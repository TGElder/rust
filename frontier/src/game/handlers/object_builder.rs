use super::*;
use commons::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

pub struct ObjectBuilder {
    command_tx: Sender<GameCommand>,
    world_coord: Option<WorldCoord>,
    bindings: ObjectBuilderBindings,
}

struct ObjectBuilderBindings {
    build_house: Button,
    build_farm: Button,
}

impl ObjectBuilder {
    pub fn new(command_tx: Sender<GameCommand>) -> ObjectBuilder {
        ObjectBuilder {
            command_tx,
            world_coord: None,
            bindings: ObjectBuilderBindings {
                build_house: Button::Key(VirtualKeyCode::H),
                build_farm: Button::Key(VirtualKeyCode::F),
            },
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn build_object(&mut self, object: WorldObject, game_state: &GameState) {
        if let Some(WorldCoord { x, y, .. }) = self.world_coord {
            let position = v2(x.floor() as usize, y.floor() as usize);
            if let Some(cell) = game_state.world.get_cell(&position) {
                let command = if cell.object == WorldObject::None {
                    Some(GameCommand::UpdateObject {
                        object,
                        position,
                        build: true,
                    })
                } else if cell.object == object {
                    Some(GameCommand::UpdateObject {
                        object,
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

impl GameEventConsumer for ObjectBuilder {
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
            if button == &self.bindings.build_house {
                self.build_object(WorldObject::House, &game_state);
            }
            if button == &self.bindings.build_farm {
                self.build_object(WorldObject::Farm, &game_state);
            }
        }
        CaptureEvent::No
    }
}
