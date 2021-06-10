use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use commons::async_std::task::sleep;
use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::process::Step;
use commons::{unsafe_ordering, v2, V2};
use isometric::{Button, ElementState, Event, VirtualKeyCode};

use crate::avatar::{Avatar, AvatarTravelDuration, Frame, Journey, Rotation};
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::has::HasParameters;
use crate::traits::{Micros, SelectedAvatar, UpdateAvatarJourney, WithWorld};
use crate::travel_duration::TravelDuration;
use crate::world::{World, WorldCell};

pub struct RiverExplorer<T> {
    cx: T,
    active: bool,
    travel_duration: Arc<AvatarTravelDuration>,
    parameters: RiverExplorerParameters,
}

pub struct RiverExplorerParameters {
    pub refresh_interval: Duration,
    pub binding: Button,
    pub min_navigable_river_width: f32,
}

impl Default for RiverExplorerParameters {
    fn default() -> RiverExplorerParameters {
        RiverExplorerParameters {
            refresh_interval: Duration::from_millis(100),
            binding: Button::Key(VirtualKeyCode::X),
            min_navigable_river_width: 0.1,
        }
    }
}

impl<T> RiverExplorer<T>
where
    T: HasParameters + Micros + SelectedAvatar + UpdateAvatarJourney + WithWorld + Send + Sync,
{
    pub fn new(
        cx: T,
        parameters: RiverExplorerParameters,
        travel_duration: Arc<AvatarTravelDuration>,
    ) -> RiverExplorer<T> {
        RiverExplorer {
            cx,
            active: false,
            travel_duration,
            parameters,
        }
    }

    async fn explore(&self) {
        let (name, journey) = match self.cx.selected_avatar().await {
            Some(Avatar {
                name,
                journey: Some(journey),
                ..
            }) => (name, journey),
            _ => return,
        };

        let micros = self.cx.micros().await;
        if !journey.done(&micros) {
            return;
        }

        let Frame {
            position, rotation, ..
        } = journey.final_frame();

        let next_direction = unwrap_or!(self.get_next_direction(position, rotation).await, return);

        let new_journey = self.get_new_journey(journey, next_direction, micros).await;

        self.cx
            .update_avatar_journey(&name, Some(new_journey))
            .await;
    }

    async fn get_next_direction(
        &self,
        position: &V2<usize>,
        rotation: &Rotation,
    ) -> Option<Rotation> {
        let grid_width = self.cx.parameters().width;
        let behind = unwrap_or!(behind(&position, &rotation, &grid_width), return None);
        let forward_candidates = possible_directions(&rotation);
        self.find_valid_direction(&position, &behind, forward_candidates)
            .await
    }

    async fn find_valid_direction(
        &self,
        current_position: &V2<usize>,
        position_behind: &V2<usize>,
        possible_directions: Vec<Rotation>,
    ) -> Option<Rotation> {
        let min_navigable_river_width = self.parameters.min_navigable_river_width;
        self.cx
            .with_world(|world| {
                let current_cell = unwrap_or!(world.get_cell(current_position), return None);
                if current_cell.river.longest_side() < min_navigable_river_width {
                    return None;
                }

                let direction_to_cell = lookup_cells(world, current_cell, possible_directions);

        

                let valid_direction_to_cells = direction_to_cell
                    .filter(|(_, cell)| cell.river.longest_side() >= min_navigable_river_width)
                    .filter(|(_, next_cell)| {
                        self.travel_duration
                            .get_duration(world, &current_cell.position, &next_cell.position)
                            .is_some()
                    })
                    .collect::<HashMap<_, _>>();

                if valid_direction_to_cells.is_empty() {
                    None
                } else if valid_direction_to_cells.len() == 1 {
                    valid_direction_to_cells
                        .into_iter()
                        .next()
                        .map(|(direction, _)| direction)
                } else {
                    choose_from_multiple_valid_directions(
                        valid_direction_to_cells,
                        current_cell,
                        &world.get_cell(position_behind),
                    )
                }
            })
            .await
    }

    async fn get_new_journey(
        &self,
        journey: Journey,
        next_direction: Rotation,
        micros: u128,
    ) -> Journey {
        let forward_path = journey.then_rotate_to(next_direction).forward_path();
        let new_journey = self
            .cx
            .with_world(|world| {
                Journey::new(
                    world,
                    forward_path,
                    self.travel_duration.as_ref(),
                    self.travel_duration.travel_mode_fn(),
                    micros,
                )
            })
            .await;
        new_journey
    }
}

fn possible_directions(rotation: &Rotation) -> Vec<Rotation> {
    match rotation {
        Rotation::Left => vec![Rotation::Down, Rotation::Left, Rotation::Up],
        Rotation::Up => vec![Rotation::Left, Rotation::Up, Rotation::Right],
        Rotation::Right => vec![Rotation::Up, Rotation::Right, Rotation::Down],
        Rotation::Down => vec![Rotation::Right, Rotation::Down, Rotation::Left],
    }
}

fn offset(direction: &Rotation) -> V2<i32> {
    match direction {
        Rotation::Left => v2(-1, 0),
        Rotation::Up => v2(0, 1),
        Rotation::Right => v2(1, 0),
        Rotation::Down => v2(0, -1),
    }
}

fn behind(position: &V2<usize>, rotation: &Rotation, grid_width: &usize) -> Option<V2<usize>> {
    let behind = v2(position.x as i32, position.y as i32) + offset(&rotation) * -1;
    if behind.x >= 0
        && behind.y >= 0
        && (behind.x as usize) < *grid_width
        && (behind.y as usize) < *grid_width
    {
        Some(v2(behind.x as usize, behind.y as usize))
    } else {
        None
    }
}

fn lookup_cells<'a>(
    world: &'a World,
    current_cell: &'a WorldCell,
    directions: Vec<Rotation>,
) -> impl Iterator<Item = (Rotation, &'a WorldCell)> {
    directions.into_iter().flat_map(move |direction| {
        world
            .offset(&current_cell.position, offset(&direction))
            .and_then(|position| world.get_cell(&position))
            .map(|cell| (direction, cell))
    })
}

fn choose_from_multiple_valid_directions(
    direction_to_cell: HashMap<Rotation, &WorldCell>,
    current_cell: &WorldCell,
    cell_behind: &Option<&WorldCell>,
) -> Option<Rotation> {
    let mut ordering_to_direction_to_cell: HashMap<Ordering, HashMap<Rotation, &WorldCell>> = hashmap! {};
    for (direction, next_cell) in direction_to_cell {
        let ordering = unsafe_ordering(&current_cell.river.longest_side(), &next_cell.river.longest_side());
        ordering_to_direction_to_cell.entry(ordering).or_default().insert(direction, next_cell);
    }
    let direction_to_cell = if ordering_to_direction_to_cell.len() == 1 {
        ordering_to_direction_to_cell
            .into_iter()
            .next()
            .map(|(_, direction_to_cell)| direction_to_cell)
    } else {
        match cell_behind {
            Some(cell_behind) => ordering_to_direction_to_cell.remove(&unsafe_ordering(
                &cell_behind.river.longest_side(),
                &current_cell.river.longest_side(),
            )),
            None => None,
        }
    };
    direction_to_cell
        .into_iter()
        .flatten()
        .max_by(|(_, cell_a), (_, cell_b)| {
            unsafe_ordering(&cell_a.river.longest_side(), &cell_b.river.longest_side())
        })
        .map(|(direction, _)| direction)
}

#[async_trait]
impl<T> Step for RiverExplorer<T>
where
    T: HasParameters
        + Micros
        + SelectedAvatar
        + UpdateAvatarJourney
        + WithWorld
        + Send
        + Sync
        + 'static,
{
    async fn step(&mut self) {
        if self.active {
            self.explore().await;
        }

        sleep(self.parameters.refresh_interval).await;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for RiverExplorer<T>
where
    T: HasParameters
        + Micros
        + SelectedAvatar
        + UpdateAvatarJourney
        + WithWorld
        + Send
        + Sync
        + 'static,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if *button == self.parameters.binding && !modifiers.alt() && modifiers.ctrl() {
                self.active = !self.active;
            }
        }
        Capture::No
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use commons::junction::PositionJunction;
    use commons::M;
    use futures::executor::block_on;
    use isometric::Color;

    use crate::avatar::{AvatarTravelParams, Vehicle};
    use crate::parameters::Parameters;

    use super::*;

    struct Cx {
        avatar: Mutex<Avatar>,
        parameters: Parameters,
        world: Mutex<World>,
    }

    impl Default for Cx {
        fn default() -> Self {
            let mut world = World::new(M::from_element(3, 3, 1.0), 0.0);
            world.reveal_all();
            Cx {
                avatar: Mutex::new(Avatar {
                    name: "".to_string(),
                    journey: Some(Journey::stationary(
                        &world,
                        v2(1, 1),
                        Vehicle::None,
                        Rotation::Right,
                    )),
                    color: Color::transparent(),
                    skin_color: Color::transparent(),
                }),
                parameters: Parameters{
                    width: 3,
                    ..Parameters::default()
                },
                world: Mutex::new(world),
            }
        }
    }

    impl HasParameters for Cx {
        fn parameters(&self) -> &Parameters {
            &self.parameters
        }
    }

    #[async_trait]
    impl Micros for Cx {
        async fn micros(&self) -> u128 {
            0
        }
    }

    #[async_trait]
    impl SelectedAvatar for Cx {
        async fn selected_avatar(&self) -> Option<Avatar> {
            Some(self.avatar.lock().unwrap().clone())
        }
    }

    #[async_trait]
    impl UpdateAvatarJourney for Cx {
        async fn update_avatar_journey(&self, _: &str, journey: Option<Journey>) {
            self.avatar.lock().unwrap().journey = journey;
        }
    }

    #[async_trait]
    impl WithWorld for Cx {
        async fn with_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&World) -> O + Send,
        {
            function(&self.world.lock().unwrap())
        }

        async fn mut_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut World) -> O + Send,
        {
            function(&mut self.world.lock().unwrap())
        }
    }

    fn avatar_travel_duration() -> Arc<AvatarTravelDuration> {
        Arc::new(AvatarTravelDuration::new(AvatarTravelParams::default()))
    }

    #[test]
    fn single_candidate() {
        // Given
        let cx = Cx::default();

        let mut river_1 = PositionJunction::new(v2(1, 1));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;

        let mut river_2 = PositionJunction::new(v2(2, 1));
        river_2.junction.horizontal.width = 1.0;
        river_2.junction.horizontal.from = true;
        river_2.junction.horizontal.to = true;

        {
            let mut world = cx.world.lock().unwrap();
            world.add_river(river_1);
            world.add_river(river_2);
        }

        let parameters = RiverExplorerParameters{
            min_navigable_river_width: 0.1,
            ..RiverExplorerParameters::default()
        };

        let travel_duration = avatar_travel_duration();

        let river_explorer = RiverExplorer::new(cx, parameters, travel_duration.clone());

        // When
        block_on(river_explorer.explore());

        // Then
        assert_eq!(river_explorer.cx.avatar.lock().unwrap().journey, Some(Journey::new(
            &river_explorer.cx.world.lock().unwrap(),
            vec![v2(1, 1), v2(2, 1)],
            travel_duration.as_ref(),
            travel_duration.travel_mode_fn(),
            0,
        )));
    }

    #[test]
    fn multiple_candidates_all_upstream() {
         // Given
         let cx = Cx::default();

         let mut river_1 = PositionJunction::new(v2(1, 1));
         river_1.junction.vertical.width = 1.0;
         river_1.junction.vertical.from = true;
         river_1.junction.vertical.to = true;
 
         let mut river_2 = PositionJunction::new(v2(1, 2));
         river_2.junction.vertical.width = 1.5;
         river_2.junction.vertical.from = true;
         river_2.junction.vertical.to = true;

         let mut river_3 = PositionJunction::new(v2(1, 0));
         river_3.junction.vertical.width = 2.0;
         river_3.junction.vertical.from = true;
         river_3.junction.vertical.to = true;

 
         {
             let mut world = cx.world.lock().unwrap();
             world.add_river(river_1);
             world.add_river(river_2);
             world.add_river(river_3);
         }
 
         let parameters = RiverExplorerParameters{
             min_navigable_river_width: 0.1,
             ..RiverExplorerParameters::default()
         };
 
         let travel_duration = avatar_travel_duration();
 
         let river_explorer = RiverExplorer::new(cx, parameters, travel_duration.clone());
 
         // When
         block_on(river_explorer.explore());
 
         // Then
         assert_eq!(river_explorer.cx.avatar.lock().unwrap().journey, Some(Journey::new(
             &river_explorer.cx.world.lock().unwrap(),
             vec![v2(1, 1), v2(1, 0)],
             travel_duration.as_ref(),
             travel_duration.travel_mode_fn(),
             0,
         )));
    }

    #[test]
    fn mixed_candidates_moving_downstream() {
          // Given
          let cx = Cx::default();

          let mut river_1 = PositionJunction::new(v2(0, 1));
          river_1.junction.horizontal.width = 2.0;
          river_1.junction.horizontal.from = true;
          river_1.junction.horizontal.to = true;

          let mut river_2 = PositionJunction::new(v2(1, 1));
          river_2.junction.horizontal.width = 1.0;
          river_2.junction.horizontal.from = true;
          river_2.junction.horizontal.to = true;
          river_2.junction.vertical.width = 1.0;
          river_2.junction.vertical.from = true;
          river_2.junction.vertical.to = true;
  
          let mut river_3 = PositionJunction::new(v2(1, 2));
          river_3.junction.vertical.width = 3.0;
          river_3.junction.vertical.from = true;
          river_3.junction.vertical.to = true;
 
          let mut river_4 = PositionJunction::new(v2(1, 0));
          river_4.junction.vertical.width = 0.5;
          river_4.junction.vertical.from = true;
          river_4.junction.vertical.to = true;

          let mut river_5 = PositionJunction::new(v2(2, 1));
          river_5.junction.horizontal.width = 4.0;
          river_5.junction.horizontal.from = true;
          river_5.junction.horizontal.to = true;
  
          {
              let mut world = cx.world.lock().unwrap();
              world.add_river(river_1);
              world.add_river(river_2);
              world.add_river(river_3);
              world.add_river(river_4);
              world.add_river(river_5);
          }
  
          let parameters = RiverExplorerParameters{
              min_navigable_river_width: 0.1,
              ..RiverExplorerParameters::default()
          };
  
          let travel_duration = avatar_travel_duration();
  
          let river_explorer = RiverExplorer::new(cx, parameters, travel_duration.clone());
  
          // When
          block_on(river_explorer.explore());
  
          // Then
          assert_eq!(river_explorer.cx.avatar.lock().unwrap().journey, Some(Journey::new(
              &river_explorer.cx.world.lock().unwrap(),
              vec![v2(1, 1), v2(1, 0)],
              travel_duration.as_ref(),
              travel_duration.travel_mode_fn(),
              0,
          )));
    }

    #[test]
    fn mixed_candidates_moving_upstream() {
          // Given
          let cx = Cx::default();

          let mut river_1 = PositionJunction::new(v2(0, 1));
          river_1.junction.horizontal.width = 0.5;
          river_1.junction.horizontal.from = true;
          river_1.junction.horizontal.to = true;

          let mut river_2 = PositionJunction::new(v2(1, 1));
          river_2.junction.horizontal.width = 1.0;
          river_2.junction.horizontal.from = true;
          river_2.junction.horizontal.to = true;
          river_2.junction.vertical.width = 1.0;
          river_2.junction.vertical.from = true;
          river_2.junction.vertical.to = true;
  
          let mut river_3 = PositionJunction::new(v2(1, 2));
          river_3.junction.vertical.width = 3.0;
          river_3.junction.vertical.from = true;
          river_3.junction.vertical.to = true;
 
          let mut river_4 = PositionJunction::new(v2(1, 0));
          river_4.junction.vertical.width = 0.5;
          river_4.junction.vertical.from = true;
          river_4.junction.vertical.to = true;

          let mut river_5 = PositionJunction::new(v2(2, 1));
          river_5.junction.horizontal.width = 4.0;
          river_5.junction.horizontal.from = true;
          river_5.junction.horizontal.to = true;
  
          {
              let mut world = cx.world.lock().unwrap();
              world.add_river(river_1);
              world.add_river(river_2);
              world.add_river(river_3);
              world.add_river(river_4);
              world.add_river(river_5);
          }
  
          let parameters = RiverExplorerParameters{
              min_navigable_river_width: 0.1,
              ..RiverExplorerParameters::default()
          };
  
          let travel_duration = avatar_travel_duration();
  
          let river_explorer = RiverExplorer::new(cx, parameters, travel_duration.clone());
  
          // When
          block_on(river_explorer.explore());
  
          // Then
          assert_eq!(river_explorer.cx.avatar.lock().unwrap().journey, Some(Journey::new(
              &river_explorer.cx.world.lock().unwrap(),
              vec![v2(1, 1), v2(2, 1)],
              travel_duration.as_ref(),
              travel_duration.travel_mode_fn(),
              0,
          )));
    }

    #[test]
    fn no_candidates() {
        // Given
        let cx = Cx::default();

        let mut river_1 = PositionJunction::new(v2(1, 1));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;

        let mut river_2 = PositionJunction::new(v2(0, 1)); // Behind
        river_2.junction.horizontal.width = 1.0;
        river_2.junction.horizontal.from = true;
        river_2.junction.horizontal.to = true;

        {
            let mut world = cx.world.lock().unwrap();
            world.add_river(river_1);
            world.add_river(river_2);
        }

        let parameters = RiverExplorerParameters{
            min_navigable_river_width: 0.1,
            ..RiverExplorerParameters::default()
        };

        let travel_duration = avatar_travel_duration();

        let river_explorer = RiverExplorer::new(cx, parameters, travel_duration.clone());

        // When
        block_on(river_explorer.explore());

        // Then
        assert_eq!(river_explorer.cx.avatar.lock().unwrap().journey, Some(Journey::stationary(
            &river_explorer.cx.world.lock().unwrap(),
            v2(1, 1),
            Vehicle::None,
            Rotation::Right,
        )));
    }

    #[test]
    fn avatar_not_in_river() {
         // Given
         let cx = Cx::default();

         let mut river_1 = PositionJunction::new(v2(2, 1));
         river_1.junction.horizontal.width = 1.0;
         river_1.junction.horizontal.from = true;
         river_1.junction.horizontal.to = true;
 
         {
             let mut world = cx.world.lock().unwrap();
             world.add_river(river_1);
         }
 
         let parameters = RiverExplorerParameters{
             min_navigable_river_width: 0.1,
             ..RiverExplorerParameters::default()
         };
 
         let travel_duration = avatar_travel_duration();
 
         let river_explorer = RiverExplorer::new(cx, parameters, travel_duration.clone());
 
         // When
         block_on(river_explorer.explore());
 
         // Then
         assert_eq!(river_explorer.cx.avatar.lock().unwrap().journey, Some(Journey::stationary(
             &river_explorer.cx.world.lock().unwrap(),
             v2(1, 1),
             Vehicle::None,
             Rotation::Right,
         )));
    }

    #[test]
    fn should_not_cut_inside_u_bend() {
        // Given
        let cx = Cx::default();

        let mut river_1 = PositionJunction::new(v2(1, 1));
        river_1.junction.horizontal.width = 0.5;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;

        let mut river_2 = PositionJunction::new(v2(2, 1));
        river_2.junction.horizontal.width = 1.0;
        river_2.junction.horizontal.from = true;
        river_2.junction.horizontal.to = true;
        river_2.junction.vertical.width = 1.0;
        river_2.junction.vertical.from = true;
        river_2.junction.vertical.to = true;

        let mut river_3 = PositionJunction::new(v2(2, 2));
        river_3.junction.horizontal.width = 2.0;
        river_3.junction.horizontal.from = true;
        river_3.junction.horizontal.to = true;
        river_3.junction.vertical.width = 2.0;
        river_3.junction.vertical.from = true;
        river_3.junction.vertical.to = true;

        let mut river_4 = PositionJunction::new(v2(1, 2));
        river_4.junction.horizontal.width = 4.0;
        river_4.junction.horizontal.from = true;
        river_4.junction.horizontal.to = true;

        {
            let mut world = cx.world.lock().unwrap();
            world.add_river(river_1);
            world.add_river(river_2);
            world.add_river(river_3);
            world.add_river(river_4);
        }

        let parameters = RiverExplorerParameters{
            min_navigable_river_width: 0.1,
            ..RiverExplorerParameters::default()
        };

        let travel_duration = avatar_travel_duration();

        let river_explorer = RiverExplorer::new(cx, parameters, travel_duration.clone());

        // When
        block_on(river_explorer.explore());

        // Then
        assert_eq!(river_explorer.cx.avatar.lock().unwrap().journey, Some(Journey::new(
            &river_explorer.cx.world.lock().unwrap(),
            vec![v2(1, 1), v2(2, 1)],
            travel_duration.as_ref(),
            travel_duration.travel_mode_fn(),
            0,
        )));
    }

}
