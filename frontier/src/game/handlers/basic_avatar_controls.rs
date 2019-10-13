use super::*;
use crate::avatar::*;
use crate::travel_duration::TravelDuration;
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
    travel_duration: Option<AvatarTravelDuration>,
    bindings: BasicAvatarBindings,
}

impl BasicAvatarControls {
    pub fn new(command_tx: Sender<GameCommand>) -> BasicAvatarControls {
        BasicAvatarControls {
            command_tx,
            travel_duration: None,
            bindings: BasicAvatarBindings::default(),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.travel_duration = Some(AvatarTravelDuration::from_params(
            &game_state.params.avatar_travel,
        ));
    }

    fn walk_forward(&mut self, game_state: &GameState) {
        if let Some(travel_duration) = &self.travel_duration {
            if let Some(path) = game_state.avatar_state.forward_path() {
                let start_at = game_state.game_micros;
                if travel_duration
                    .get_duration(&game_state.world, &path[0], &path[1])
                    .is_some()
                {
                    if let Some(new_state) = game_state.avatar_state.walk_positions(
                        &game_state.world,
                        path,
                        travel_duration,
                        start_at,
                    ) {
                        self.command_tx
                            .send(GameCommand::UpdateAvatar(new_state))
                            .unwrap();
                    }
                }
            }
        }
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
