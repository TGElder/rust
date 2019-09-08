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
    pathfinder: Option<Pathfinder<AvatarTravelDuration>>,
    bindings: BasicAvatarBindings,
}

impl BasicAvatarControls {
    pub fn new(command_tx: Sender<GameCommand>) -> BasicAvatarControls {
        BasicAvatarControls {
            command_tx,
            pathfinder: None,
            bindings: BasicAvatarBindings::default(),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.pathfinder = Some(Pathfinder::new(
            &game_state.world,
            AvatarTravelDuration::from_params(&game_state.params.avatar_travel),
        ));
    }

    fn reset_pathfinder(&mut self, game_state: &GameState) {
        if let Some(pathfinder) = &mut self.pathfinder {
            pathfinder.compute_network(&game_state.world);
        }
    }

    fn walk_forward(&mut self, game_state: &GameState) {
        if let Some(ref pathfinder) = self.pathfinder {
            if let Some(new_state) = game_state.avatar_state.walk_forward(
                &game_state.world,
                pathfinder,
                game_state.game_micros,
            ) {
                self.command_tx
                    .send(GameCommand::UpdateAvatar(new_state))
                    .unwrap();
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

    fn update_pathfinder_with_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        if let Some(pathfinder) = &mut self.pathfinder {
            for cell in cells {
                pathfinder.update_node(&game_state.world, cell);
            }
        }
    }

    fn update_pathfinder_with_roads(&mut self, game_state: &GameState, result: &RoadBuilderResult) {
        if let Some(pathfinder) = &mut self.pathfinder {
            result.update_pathfinder(&game_state.world, pathfinder);
        }
    }
}

impl GameEventConsumer for BasicAvatarControls {
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
