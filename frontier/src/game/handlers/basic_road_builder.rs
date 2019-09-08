use super::*;
use crate::road_builder::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

pub struct BasicRoadBuilder {
    command_tx: Sender<GameCommand>,
    road_builder: Option<RoadBuilder<AvatarTravelDuration>>,
    binding: Button,
}

impl BasicRoadBuilder {
    pub fn new(command_tx: Sender<GameCommand>) -> BasicRoadBuilder {
        BasicRoadBuilder {
            command_tx,
            road_builder: None,
            binding: Button::Key(VirtualKeyCode::R),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.road_builder = Some(RoadBuilder::<AvatarTravelDuration>::new(
            &game_state.world,
            AvatarTravelDuration::from_params(&game_state.params.avatar_travel),
        ));
    }

    fn reset_pathfinder(&mut self, game_state: &GameState) {
        if let Some(road_builder) = &mut self.road_builder {
            road_builder.pathfinder().compute_network(&game_state.world);
        }
    }

    fn build_road(&mut self, game_state: &GameState) {
        if let Some(road_builder) = &mut self.road_builder {
            let result = road_builder.build_forward(&game_state.world, &game_state.avatar_state);
            if let Some(result) = result {
                self.command_tx
                    .send(GameCommand::UpdateRoads(result))
                    .unwrap();
                let new_avatar_state = game_state
                    .avatar_state
                    .walk_forward(
                        &game_state.world,
                        road_builder.pathfinder(),
                        game_state.game_micros,
                    )
                    .unwrap();
                self.command_tx
                    .send(GameCommand::UpdateAvatar(new_avatar_state))
                    .unwrap();
            }
        }
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

impl GameEventConsumer for BasicRoadBuilder {
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
            if button == &self.binding {
                self.build_road(game_state);
            }
        }
        CaptureEvent::No
    }
}
