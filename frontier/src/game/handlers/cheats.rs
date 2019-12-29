use super::*;
use crate::shore_start::*;
use commons::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::default::Default;

pub struct CheatBindings {
    reveal_all: Button,
    move_avatar: Button,
    remove_avatar: Button,
    add_avatars: Button,
}

impl Default for CheatBindings {
    fn default() -> CheatBindings {
        CheatBindings {
            reveal_all: Button::Key(VirtualKeyCode::V),
            move_avatar: Button::Key(VirtualKeyCode::H),
            remove_avatar: Button::Key(VirtualKeyCode::R),
            add_avatars: Button::Key(VirtualKeyCode::A),
        }
    }
}

pub struct Cheats {
    command_tx: Sender<GameCommand>,
    bindings: CheatBindings,
    world_coord: Option<WorldCoord>,
}

impl Cheats {
    pub fn new(command_tx: Sender<GameCommand>) -> Cheats {
        Cheats {
            command_tx,
            bindings: CheatBindings::default(),
            world_coord: None,
        }
    }
    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn reveal_all(&mut self, _: &GameState) {
        self.command_tx
            .send(GameCommand::VisitCells(CellSelection::All))
            .unwrap();
        self.command_tx
            .send(GameCommand::RevealCells(CellSelection::All))
            .unwrap();
    }

    fn move_avatar(&mut self, game_state: &GameState) {
        if let Some(WorldCoord { x, y, .. }) = self.world_coord {
            if let Some(name) = &game_state.selected_avatar {
                let new_state = AvatarState::Stationary {
                    position: v2(x.round() as usize, y.round() as usize),
                    rotation: Rotation::Down,
                };
                self.command_tx
                    .send(GameCommand::UpdateAvatar {
                        name: name.to_string(),
                        new_state,
                    })
                    .unwrap();
            }
        };
    }

    fn remove_avatar(&mut self, game_state: &GameState) {
        if let Some(name) = &game_state.selected_avatar {
            self.command_tx
                .send(GameCommand::UpdateAvatar {
                    name: name.to_string(),
                    new_state: AvatarState::Absent,
                })
                .unwrap();
        }
    }

    fn add_avatars(&mut self, game_state: &GameState) {
        const AVATARS: usize = 100;
        let base_index = game_state.avatars.len();
        println!("Adding {} avatars to existing {}", AVATARS, base_index);
        let mut rng = rand::thread_rng();
        random_avatar_states(&game_state.world, &mut rng, AVATARS)
            .into_iter()
            .enumerate()
            .for_each(|(i, state)| {
                let name = (base_index + i).to_string();
                self.command_tx
                    .send(GameCommand::AddAvatar {
                        name: name.clone(),
                        avatar: Avatar {
                            name,
                            state,
                            farm: None,
                        },
                    })
                    .unwrap()
            });
    }
}

impl GameEventConsumer for Cheats {
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
            } else if button == &self.bindings.add_avatars {
                self.add_avatars(game_state)
            }
        }
        CaptureEvent::No
    }
}
