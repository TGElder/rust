use super::*;
use crate::road_builder::*;
use crate::travel_duration::*;
use commons::scale::*;
use commons::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::default::Default;
use std::time::Duration;

pub struct PathfindingRoadBuilderParams {
    max_gradient: f32,
    cost_at_level: f32,
    cost_at_max_gradient: f32,
    cost_on_existing_road: u64,
    binding: Button,
}

impl Default for PathfindingRoadBuilderParams {
    fn default() -> PathfindingRoadBuilderParams {
        PathfindingRoadBuilderParams {
            max_gradient: 0.5,
            cost_at_level: 575.0,
            cost_at_max_gradient: 925.0,
            cost_on_existing_road: 100,
            binding: Button::Key(VirtualKeyCode::X),
        }
    }
}

pub struct PathfindingRoadBuilder {
    command_tx: Sender<GameCommand>,
    road_builder: Option<RoadBuilder<AutoRoadTravelDuration>>,
    world_coord: Option<WorldCoord>,
    params: PathfindingRoadBuilderParams,
}

impl PathfindingRoadBuilder {
    pub fn new(command_tx: Sender<GameCommand>) -> PathfindingRoadBuilder {
        PathfindingRoadBuilder {
            command_tx,
            road_builder: None,
            world_coord: None,
            params: PathfindingRoadBuilderParams::default(),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.road_builder = Some(RoadBuilder::<AutoRoadTravelDuration>::new(
            &game_state.world,
            AutoRoadTravelDuration::new(
                GradientTravelDuration::boxed(
                    Scale::new(
                        (-self.params.max_gradient, self.params.max_gradient),
                        (self.params.cost_at_level, self.params.cost_at_max_gradient),
                    ),
                    true,
                ),
                ConstantTravelDuration::boxed(Duration::from_millis(
                    self.params.cost_on_existing_road,
                )),
            ),
        ));
    }

    fn reset_pathfinder(&mut self, game_state: &GameState) {
        if let Some(road_builder) = &mut self.road_builder {
            road_builder.pathfinder().compute_network(&game_state.world);
        }
    }

    fn build_road(&mut self, game_state: &GameState) {
        if let (Some(WorldCoord { x, y, .. }), Some(road_builder)) =
            (self.world_coord, &mut self.road_builder)
        {
            let target = &v2(x.round() as usize, y.round() as usize);
            let result = road_builder.auto_build_road(&game_state.avatar_state, &target);
            if let Some(result) = result {
                self.command_tx
                    .send(GameCommand::UpdateRoads(result))
                    .unwrap();
            }
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn update_pathfinder_with_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        if let Some(road_builder) = &mut self.road_builder {
            for cell in cells {
                road_builder
                    .pathfinder()
                    .update_node(&game_state.world, cell);
            }
        }
    }

    fn update_pathfinder_with_roads(&mut self, game_state: &GameState, result: &RoadBuilderResult) {
        if let Some(road_builder) = &mut self.road_builder {
            result.update_pathfinder(&game_state.world, road_builder.pathfinder());
        }
    }
}

impl GameEventConsumer for PathfindingRoadBuilder {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::CellsRevealed(selection) => {
                match selection {
                    CellSelection::All => self.reset_pathfinder(game_state),
                    CellSelection::Some(cells) => {
                        self.update_pathfinder_with_cells(game_state, &cells)
                    }
                };
            }
            GameEvent::RoadsUpdated(result) => {
                self.update_pathfinder_with_roads(game_state, result)
            }
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.params.binding {
                self.build_road(game_state);
            }
        }
        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        }
        CaptureEvent::No
    }
}
