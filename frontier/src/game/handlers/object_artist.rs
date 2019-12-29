use super::*;
use crate::farms::FarmArtist;
use crate::houses::HouseArtist;
use commons::*;
use isometric::Color;

pub struct ObjectArtistHandler {
    command_tx: Sender<GameCommand>,
    state: Option<ObjectArtistState>,
}

struct ObjectArtistState {
    house_artist: HouseArtist,
    farm_artist: FarmArtist,
}

impl ObjectArtistHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> ObjectArtistHandler {
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
                WorldObject::Farm => Self::draw_farm(state, game_state, position),
                _ => return,
            };
            self.command_tx
                .send(GameCommand::EngineCommands(commands))
                .unwrap();
        }
    }

    fn draw_farm(
        state: &ObjectArtistState,
        game_state: &GameState,
        position: &V2<usize>,
    ) -> Vec<Command> {
        let color = game_state
            .tile_color(position)
            .map(|mut color| {
                color.a = game_state.params.artist.territory_alpha;
                color
            })
            .unwrap_or_else(Color::transparent);
        state.farm_artist.draw_farm_at(
            &game_state.world,
            game_state.params.world_gen.sea_level as f32,
            position,
            &color,
        )
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
                WorldObject::Farm => state.farm_artist.erase_farm_at(&game_state.world, position),
                _ => return,
            };
            self.command_tx
                .send(GameCommand::EngineCommands(commands))
                .unwrap();
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

    fn draw_affected(
        &mut self,
        object: &WorldObject,
        game_state: &GameState,
        positions: &[V2<usize>],
    ) {
        positions
            .iter()
            .flat_map(|position| game_state.world.expand_position(&position))
            .for_each(|position| self.draw_object(object, game_state, &position))
    }

    fn init(&mut self, game_state: &GameState) {
        self.state = Some(ObjectArtistState {
            house_artist: HouseArtist::new(game_state.params.light_direction),
            farm_artist: FarmArtist::new(),
        });
        self.draw_all(game_state);
    }

    fn territory_change(&mut self, game_state: &GameState, changes: &[TerritoryChange]) {
        let changes: Vec<V2<usize>> = changes.iter().map(|change| change.position).collect();
        self.draw_affected(&WorldObject::Farm, game_state, &changes);
    }
}

impl GameEventConsumer for ObjectArtistHandler {
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
            GameEvent::RoadsUpdated(result) => {
                self.draw_affected(&WorldObject::Farm, game_state, result.path())
            }
            GameEvent::TerritoryChanged(changes) => self.territory_change(game_state, changes),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
