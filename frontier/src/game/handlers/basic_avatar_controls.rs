use super::*;
use crate::avatar::*;
use crate::pathfinder::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::default::Default;

pub struct BasicAvatarBindings {
    forward: Button,
    rotate_clockwise: Button,
    rotate_anticlockwise: Button,
}

impl Default for BasicAvatarBindings {
    fn default() -> BasicAvatarBindings {
        BasicAvatarBindings {
            forward: Button::Key(VirtualKeyCode::W),
            rotate_clockwise: Button::Key(VirtualKeyCode::D),
            rotate_anticlockwise: Button::Key(VirtualKeyCode::A),
        }
    }
}

pub struct BasicAvatarControls {
    command_tx: Sender<GameCommand>,
    pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    bindings: BasicAvatarBindings,
}

impl BasicAvatarControls {
    pub fn new(
        command_tx: Sender<GameCommand>,
        pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    ) -> BasicAvatarControls {
        BasicAvatarControls {
            command_tx,
            pathfinder_tx: pathfinder_tx,
            bindings: BasicAvatarBindings::default(),
        }
    }

    fn walk_forward(&mut self, game_state: &GameState) {
        if let Some(path) = game_state.avatar_state.forward_path() {
            let start_at = game_state.game_micros;
            let function: Box<Fn(&Pathfinder<AvatarTravelDuration>) -> Vec<GameCommand> + Send> =
                Box::new(move |pathfinder| {
                    if let Some(positions) = pathfinder.find_path(&path[0], &path[1]) {
                        if positions.len() == 2 {
                            return vec![GameCommand::WalkPositions {
                                positions,
                                start_at,
                            }];
                        }
                    }
                    vec![]
                });
            self.pathfinder_tx
                .send(PathfinderCommand::Use(function))
                .unwrap();
        };
    }

    fn rotate_clockwise(&mut self, game_state: &GameState) {
        if let Some(new_state) = game_state.avatar_state.rotate_clockwise() {
            self.command_tx
                .send(GameCommand::UpdateAvatar(new_state))
                .unwrap();
        }
    }

    fn rotate_anticlockwise(&mut self, game_state: &GameState) {
        if let Some(new_state) = game_state.avatar_state.rotate_anticlockwise() {
            self.command_tx
                .send(GameCommand::UpdateAvatar(new_state))
                .unwrap();
        }
    }
}

impl GameEventConsumer for BasicAvatarControls {
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
            if button == &self.bindings.forward {
                self.walk_forward(&game_state)
            } else if button == &self.bindings.rotate_clockwise {
                self.rotate_clockwise(&game_state)
            } else if button == &self.bindings.rotate_anticlockwise {
                self.rotate_anticlockwise(&game_state)
            };
        }
        CaptureEvent::No
    }
}
