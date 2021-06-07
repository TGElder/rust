use std::{sync::Arc, time::Duration};

use commons::{
    async_std::task::sleep, async_trait::async_trait, grid::Grid, log::info, process::Step,
    unsafe_ordering, v2, V2,
};
use isometric::{Button, ElementState, Event, VirtualKeyCode};

use crate::{
    avatar::{Avatar, AvatarTravelDuration, Frame, Journey, Rotation},
    system::{Capture, HandleEngineEvent},
    traits::{has::HasParameters, Micros, SelectedAvatar, UpdateAvatarJourney, WithWorld},
    travel_duration::TravelDuration,
};

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
        let forward_candidates = forward_candidates(&rotation);
        self.choose_candidate(&behind, position, forward_candidates)
            .await
    }

    async fn choose_candidate(
        &self,
        behind: &V2<usize>,
        position: &V2<usize>,
        forward_candidates: Vec<Rotation>,
    ) -> Option<Rotation> {
        let min_navigable_river_width = self.parameters.min_navigable_river_width;
        self.cx
            .with_world(|world| {
                let position = unwrap_or!(world.get_cell(position), return None);
                if position.river.longest_side() < min_navigable_river_width {
                    return None;
                }
                let behind = unwrap_or!(world.get_cell(behind), return None);
                forward_candidates
                    .into_iter()
                    .flat_map(|candidate| {
                        world
                            .offset(&position.position, offset(&candidate))
                            .and_then(|position| world.get_cell(&position))
                            .map(|cell| (candidate, cell))
                    })
                    .filter(|(_, forward)| {
                        self.travel_duration
                            .get_duration(world, &position.position, &forward.position)
                            .is_some()
                    })
                    .filter(|(_, forward)| {
                        forward.river.longest_side() >= min_navigable_river_width
                    })
                    .filter(|(_, forward)| {
                        (behind.elevation <= position.elevation)
                            && (position.elevation <= forward.elevation)
                            || (behind.elevation >= position.elevation)
                                && (position.elevation >= forward.elevation)
                    })
                    .max_by(|a, b| {
                        unsafe_ordering(&a.1.river.longest_side(), &b.1.river.longest_side())
                    })
                    .map(|(candidate, _)| candidate)
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

fn forward_candidates(rotation: &Rotation) -> Vec<Rotation> {
    match rotation {
        Rotation::Left => vec![Rotation::Down, Rotation::Left, Rotation::Up],
        Rotation::Up => vec![Rotation::Left, Rotation::Up, Rotation::Right],
        Rotation::Right => vec![Rotation::Up, Rotation::Right, Rotation::Down],
        Rotation::Down => vec![Rotation::Right, Rotation::Down, Rotation::Left],
    }
}

fn offset(rotation: &Rotation) -> V2<i32> {
    match rotation {
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
