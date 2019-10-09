use super::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

pub struct BasicRoadBuilder {
    pathfinder_tx: Sender<PathfinderCommand<AutoRoadTravelDuration>>,
    binding: Button,
}

impl BasicRoadBuilder {
    pub fn new(
        pathfinder_tx: Sender<PathfinderCommand<AutoRoadTravelDuration>>,
    ) -> BasicRoadBuilder {
        BasicRoadBuilder {
            pathfinder_tx: pathfinder_tx,
            binding: Button::Key(VirtualKeyCode::R),
        }
    }

    fn build_road(&mut self, game_state: &GameState) {
        if let Some(path) = game_state.avatar_state.forward_path() {
            let start_at = game_state.game_micros;
            let function: Box<Fn(&Pathfinder<AutoRoadTravelDuration>) -> Vec<GameCommand> + Send> =
                Box::new(move |pathfinder| {
                    if let Some(result) = build_road(path[0], path[1], &pathfinder) {
                        return vec![
                            GameCommand::UpdateRoads(result),
                            GameCommand::WalkPositions {
                                positions: vec![path[0], path[1]],
                                start_at,
                            },
                        ];
                    }
                    vec![]
                });
            self.pathfinder_tx
                .send(PathfinderCommand::Use(function))
                .unwrap();
        };
    }
}

impl GameEventConsumer for BasicRoadBuilder {
    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
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
