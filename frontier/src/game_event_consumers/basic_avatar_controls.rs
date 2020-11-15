use super::*;
use crate::travel_duration::TravelDuration;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::default::Default;

const NAME: &str = "basic_avatar_controls";

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
    game_tx: FnSender<Game>,
    travel_duration: Option<AvatarTravelDuration>,
    bindings: BasicAvatarBindings,
}

impl BasicAvatarControls {
    pub fn new(game_tx: &FnSender<Game>) -> BasicAvatarControls {
        BasicAvatarControls {
            game_tx: game_tx.clone_with_name(NAME),
            travel_duration: None,
            bindings: BasicAvatarBindings::default(),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.travel_duration = Some(AvatarTravelDuration::with_planned_roads_ignored(
            &game_state.params.avatar_travel,
        ));
    }

    fn walk_forward(&mut self, game_state: &GameState) {
        if let Some(travel_duration) = &self.travel_duration {
            if let Some(Avatar { name, state, .. }) = &game_state.selected_avatar() {
                if let Some(path) = state.forward_path() {
                    let start_at = game_state.game_micros;
                    if travel_duration
                        .get_duration(&game_state.world, &path[0], &path[1])
                        .is_some()
                    {
                        if let Some(new_state) = state.walk_positions(
                            &game_state.world,
                            path,
                            travel_duration,
                            start_at,
                            None,
                            None,
                        ) {
                            self.send_update_avatar_state_command(name, new_state);
                        }
                    }
                }
            }
        }
    }

    fn rotate_clockwise(&mut self, game_state: &GameState) {
        if let Some(Avatar { name, state, .. }) = &game_state.selected_avatar() {
            if let Some(new_state) = state.rotate_clockwise() {
                self.send_update_avatar_state_command(name, new_state);
            }
        }
    }

    fn rotate_anticlockwise(&mut self, game_state: &GameState) {
        if let Some(Avatar { name, state, .. }) = &game_state.selected_avatar() {
            if let Some(new_state) = state.rotate_anticlockwise() {
                self.send_update_avatar_state_command(name, new_state);
            }
        }
    }

    fn send_update_avatar_state_command(&mut self, name: &str, avatar_state: AvatarState) {
        let name = name.to_string();
        self.game_tx.send(move |game| {
            game.update_avatar_state(name.to_string(), avatar_state);
        });
    }
}

impl GameEventConsumer for BasicAvatarControls {
    fn name(&self) -> &'static str {
        NAME
    }

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
