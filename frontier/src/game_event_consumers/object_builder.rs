use super::*;
use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "object_builder_handler";

pub struct ObjectBuilder {
    game_tx: UpdateSender<Game>,
    rng: SmallRng,
    bindings: ObjectBuilderBindings,
    world_coord: Option<WorldCoord>,
}

struct ObjectBuilderBindings {
    build_farm: Button,
    demolish: Button,
}

impl ObjectBuilder {
    pub fn new(seed: u64, game_tx: &UpdateSender<Game>) -> ObjectBuilder {
        ObjectBuilder {
            game_tx: game_tx.clone_with_handle(HANDLE),
            rng: SeedableRng::seed_from_u64(seed),
            bindings: ObjectBuilderBindings {
                build_farm: Button::Key(VirtualKeyCode::F),
                demolish: Button::Key(VirtualKeyCode::U),
            },
            world_coord: None,
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|world_coord| world_coord.to_v2_floor())
    }

    fn build_object_at_cursor(&mut self, object: WorldObject) {
        if let Some(position) = self.get_position() {
            self.game_tx
                .update(move |game| game.update_object(object, position, true));
        }
    }

    fn clear_object_at_cursor(&mut self) {
        if let Some(position) = self.get_position() {
            self.game_tx.update(move |game| game.clear_object(position));
        }
    }
}

impl GameEventConsumer for ObjectBuilder {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
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
            if button == &self.bindings.build_farm {
                let rotated = self.rng.gen();
                self.build_object_at_cursor(WorldObject::Farm { rotated });
            } else if button == &self.bindings.demolish {
                self.clear_object_at_cursor();
            }
        }
        CaptureEvent::No
    }
}
