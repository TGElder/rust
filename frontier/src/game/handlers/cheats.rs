use super::*;
use commons::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::default::Default;

pub struct CheatBindings {
    reveal_all: Button,
    move_avatar: Button,
}

impl Default for CheatBindings {
    fn default() -> CheatBindings {
        CheatBindings {
            reveal_all: Button::Key(VirtualKeyCode::V),
            move_avatar: Button::Key(VirtualKeyCode::H),
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

    fn move_avatar(&mut self, _: &GameState) {
        if let Some(WorldCoord { x, y, .. }) = self.world_coord {
            let new_state = AvatarState::Stationary {
                position: v2(x.round() as usize, y.round() as usize),
                rotation: Rotation::Down,
            };
            self.command_tx
                .send(GameCommand::UpdateAvatar(new_state))
                .unwrap();
        };
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
            }
        }
        CaptureEvent::No
    }
}
