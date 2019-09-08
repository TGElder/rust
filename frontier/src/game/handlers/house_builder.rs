use super::*;
use commons::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

pub struct HouseBuilderHandler {
    command_tx: Sender<GameCommand>,
    world_coord: Option<WorldCoord>,
    binding: Button,
}

impl HouseBuilderHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> HouseBuilderHandler {
        HouseBuilderHandler {
            command_tx,
            world_coord: None,
            binding: Button::Key(VirtualKeyCode::H),
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn build_house(&mut self, game_state: &GameState) {
        if let Some(WorldCoord { x, y, .. }) = self.world_coord {
            let position = v2(x.floor() as usize, y.floor() as usize);
            if let Some(cell) = game_state.world.get_cell(&position) {
                let command = if cell.object == WorldObject::None {
                    Some(GameCommand::UpdateHouse {
                        position,
                        build: true,
                    })
                } else if cell.object == WorldObject::House {
                    Some(GameCommand::UpdateHouse {
                        position,
                        build: false,
                    })
                } else {
                    None
                };
                command
                    .into_iter()
                    .for_each(|command| self.command_tx.send(command).unwrap());
            }
        }
    }
}

impl GameEventConsumer for HouseBuilderHandler {
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
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                self.build_house(&game_state);
            }
        }
        CaptureEvent::No
    }
}
