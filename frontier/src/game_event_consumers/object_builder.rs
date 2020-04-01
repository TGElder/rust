use super::*;
use isometric::coords::*;
use isometric::Color;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "object_builder_handler";

pub struct ObjectBuilder {
    house_color: Color,
    game_tx: UpdateSender<Game>,
    world_coord: Option<WorldCoord>,
    bindings: ObjectBuilderBindings,
}

struct ObjectBuilderBindings {
    build_house: Button,
    build_farm: Button,
    demolish: Button,
}

impl ObjectBuilder {
    pub fn new(house_color: Color, game_tx: &UpdateSender<Game>) -> ObjectBuilder {
        ObjectBuilder {
            house_color,
            game_tx: game_tx.clone_with_handle(HANDLE),
            world_coord: None,
            bindings: ObjectBuilderBindings {
                build_house: Button::Key(VirtualKeyCode::H),
                build_farm: Button::Key(VirtualKeyCode::F),
                demolish: Button::Key(VirtualKeyCode::U),
            },
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|world_coord| world_coord.to_v2_floor())
    }

    fn toggle_object_at_cursor(&mut self, object: WorldObject) {
        if let Some(position) = self.get_position() {
            self.game_tx
                .update(move |game| toggle_object(game, object, position));
        }
    }

    fn demolish_at_cursor(&mut self) {
        if let Some(position) = self.get_position() {
            self.game_tx.update(move |game| game.clear_object(position));
        }
    }
}

fn toggle_object(game: &mut Game, object: WorldObject, position: V2<usize>) {
    let game_state = game.game_state();
    let cell = match game_state.world.get_cell(&position) {
        Some(cell) => cell,
        None => return,
    };
    let build = if cell.object == WorldObject::None {
        true
    } else if cell.object == object {
        false
    } else {
        return;
    };
    game.update_object(object, position, build);
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
            if button == &self.bindings.build_house {
                self.toggle_object_at_cursor(WorldObject::House(self.house_color));
            } else if button == &self.bindings.build_farm {
                self.toggle_object_at_cursor(WorldObject::Farm);
            } else if button == &self.bindings.demolish {
                self.demolish_at_cursor();
            }
        }
        CaptureEvent::No
    }
}
