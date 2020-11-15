use crate::actors::Visibility;

use super::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::default::Default;

const NAME: &str = "cheats";

pub struct CheatBindings {
    reveal_all: Button,
    move_avatar: Button,
    remove_avatar: Button,
}

impl Default for CheatBindings {
    fn default() -> CheatBindings {
        CheatBindings {
            reveal_all: Button::Key(VirtualKeyCode::V),
            move_avatar: Button::Key(VirtualKeyCode::H),
            remove_avatar: Button::Key(VirtualKeyCode::R),
        }
    }
}

pub struct Cheats {
    game_tx: FnSender<Game>,
    visibility_tx: FnSender<Visibility>,
    bindings: CheatBindings,
    world_coord: Option<WorldCoord>,
}

impl Cheats {
    pub fn new(game_tx: &FnSender<Game>, visibility_tx: &FnSender<Visibility>) -> Cheats {
        Cheats {
            game_tx: game_tx.clone_with_name(NAME),
            visibility_tx: visibility_tx.clone_with_name(NAME),
            bindings: CheatBindings::default(),
            world_coord: None,
        }
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    fn reveal_all(&mut self, _: &GameState) {
        self.game_tx.send(move |game| {
            game.reveal_all_cells(NAME);
        });
        self.visibility_tx
            .send(move |visibility| visibility.deactive());
    }

    fn move_avatar(&mut self, game_state: &GameState) {
        if let Some(world_coord) = self.world_coord {
            if let Some(name) = &game_state.selected_avatar {
                let new_state = AvatarState::Stationary {
                    position: world_coord.to_v2_round(),
                    rotation: Rotation::Down,
                };
                self.send_update_avatar_state_command(name, new_state);
            }
        };
    }

    fn remove_avatar(&mut self, game_state: &GameState) {
        if let Some(name) = &game_state.selected_avatar {
            self.send_update_avatar_state_command(name, AvatarState::Absent);
        }
    }

    fn send_update_avatar_state_command(&mut self, name: &str, avatar_state: AvatarState) {
        let name = name.to_string();
        self.game_tx.send(move |game| {
            game.update_avatar_state(name.to_string(), avatar_state);
        });
    }
}

impl GameEventConsumer for Cheats {
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        }
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: true, .. },
            ..
        } = *event
        {
            if button == &self.bindings.reveal_all {
                self.reveal_all(game_state);
            } else if button == &self.bindings.move_avatar {
                self.move_avatar(game_state);
            } else if button == &self.bindings.remove_avatar {
                self.remove_avatar(game_state)
            }
        }
        CaptureEvent::No
    }
}
