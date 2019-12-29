use super::*;
use commons::*;
use isometric::coords::*;
use isometric::Color;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use rand::prelude::*;
use rand::rngs::StdRng;

pub struct ObjectBuilder {
    command_tx: Sender<GameCommand>,
    world_coord: Option<WorldCoord>,
    bindings: ObjectBuilderBindings,
    rng: StdRng,
}

struct ObjectBuilderBindings {
    build_house: Button,
    build_farm: Button,
    demolish: Button,
}

impl ObjectBuilder {
    pub fn new(command_tx: Sender<GameCommand>) -> ObjectBuilder {
        ObjectBuilder {
            command_tx,
            world_coord: None,
            bindings: ObjectBuilderBindings {
                build_house: Button::Key(VirtualKeyCode::H),
                build_farm: Button::Key(VirtualKeyCode::F),
                demolish: Button::Key(VirtualKeyCode::U),
            },
            rng: StdRng::from_rng(rand::thread_rng()).unwrap(),
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|WorldCoord { x, y, .. }| v2(x.floor() as usize, y.floor() as usize))
    }

    fn build_object(&mut self, object: WorldObject, game_state: &GameState) {
        if let Some(position) = self.get_position() {
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

    fn demolish(&mut self, game_state: &GameState) {
        if let Some(position) = self.get_position() {
            if let Some(cell) = game_state.world.get_cell(&position) {
                if cell.object != WorldObject::None {
                    let command = GameCommand::UpdateObject {
                        object: cell.object,
                        position,
                        build: false,
                    };
                    self.command_tx.send(command).unwrap()
                }
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
                let color = Color::random(&mut self.rng, 1.0);
                self.build_object(WorldObject::House(color), &game_state);
            } else if button == &self.bindings.build_farm {
                self.build_object(WorldObject::Farm, &game_state);
            } else if button == &self.bindings.demolish {
                self.demolish(&game_state);
            }
        }
        CaptureEvent::No
    }
}
