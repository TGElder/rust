use super::*;
use crate::road_builder::*;
use crate::travel_duration::TravelDuration;
use commons::edge::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

pub struct BasicRoadBuilder {
    command_tx: Sender<GameCommand>,
    travel_duration: Option<AutoRoadTravelDuration>,
    binding: Button,
}

impl BasicRoadBuilder {
    pub fn new(command_tx: Sender<GameCommand>) -> BasicRoadBuilder {
        BasicRoadBuilder {
            command_tx,
            travel_duration: None,
            binding: Button::Key(VirtualKeyCode::R),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.travel_duration = Some(AutoRoadTravelDuration::from_params(
            &game_state.params.auto_road_travel,
        ));
    }

    fn build_road(&mut self, game_state: &GameState) {
        if let Some(travel_duration) = &self.travel_duration {
            if let Some(Avatar { name, state, .. }) = &game_state.selected_avatar() {
                if let Some(path) = state.forward_path() {
                    if travel_duration
                        .get_duration(&game_state.world, &path[0], &path[1])
                        .is_some()
                    {
                        let edge = Edge::new(path[0], path[1]);
                        let toggle = game_state.world.is_road(&edge);
                        let result = RoadBuilderResult::new(vec![path[0], path[1]], toggle);
                        self.command_tx
                            .send(GameCommand::UpdateRoads(result))
                            .unwrap();
                        let start_at = game_state.game_micros;
                        self.command_tx
                            .send(GameCommand::WalkPositions {
                                name: name.to_string(),
                                positions: path,
                                start_at,
                            })
                            .unwrap();
                    }
                }
            }
        }
    }
}

impl GameEventConsumer for BasicRoadBuilder {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Init = event {
            self.init(game_state);
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
