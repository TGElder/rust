use super::*;
use crate::houses::HouseArtist;
use commons::*;

const HANDLE: &str = "object_artist_handler";

pub struct ObjectArtistHandler {
    command_tx: Sender<Vec<Command>>,
    state: Option<ObjectArtistState>,
}

struct ObjectArtistState {
    house_artist: HouseArtist,
}

impl ObjectArtistHandler {
    pub fn new(command_tx: Sender<Vec<Command>>) -> ObjectArtistHandler {
        ObjectArtistHandler {
            command_tx,
            state: None,
        }
    }

    fn draw_object(&mut self, object: &WorldObject, game_state: &GameState, position: &V2<usize>) {
        if game_state.world.get_cell_unsafe(position).object != *object {
            return;
        }
        if let Some(state) = &self.state {
            let commands = match object {
                WorldObject::House(color) => {
                    state
                        .house_artist
                        .draw_house_at(&game_state.world, position, *color)
                }
                _ => return,
            };
            self.command_tx.send(commands).unwrap();
        }
    }

    fn erase_object(&mut self, object: &WorldObject, game_state: &GameState, position: &V2<usize>) {
        if game_state.world.get_cell_unsafe(position).object != WorldObject::None {
            return;
        }
        if let Some(state) = &self.state {
            let commands = match object {
                WorldObject::House(..) => state
                    .house_artist
                    .erase_house_at(&game_state.world, position),
                _ => return,
            };
            self.command_tx.send(commands).unwrap();
        }
    }

    fn draw_all(&mut self, game_state: &GameState) {
        for x in 0..game_state.world.width() {
            for y in 0..game_state.world.height() {
                let position = v2(x, y);
                if let Some(WorldCell { object, .. }) = game_state.world.get_cell(&position) {
                    self.draw_object(object, &game_state, &v2(x, y));
                }
            }
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.state = Some(ObjectArtistState {
            house_artist: HouseArtist::new(game_state.params.light_direction),
        });
        self.draw_all(game_state);
    }
}

impl GameEventConsumer for ObjectArtistHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(&game_state),
            GameEvent::ObjectUpdated {
                object,
                position,
                built,
            } => {
                if *built {
                    self.draw_object(object, game_state, position);
                } else {
                    self.erase_object(object, game_state, position);
                }
            }
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
