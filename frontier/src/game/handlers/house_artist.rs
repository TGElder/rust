use super::*;
use crate::houses::HouseArtist;
use commons::*;

pub struct HouseArtistHandler {
    command_tx: Sender<GameCommand>,
    house_artist: Option<HouseArtist>,
}

impl HouseArtistHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> HouseArtistHandler {
        HouseArtistHandler {
            command_tx,
            house_artist: None,
        }
    }

    fn draw_house(&mut self, game_state: &GameState, position: &V2<usize>) {
        if let Some(house_artist) = &self.house_artist {
            let commands = house_artist.draw_house_at(&game_state.world, position);
            self.command_tx
                .send(GameCommand::EngineCommands(commands))
                .unwrap();
        }
    }

    fn erase_house(&mut self, game_state: &GameState, position: &V2<usize>) {
        if let Some(house_artist) = &self.house_artist {
            let commands = house_artist.erase_house_at(&game_state.world, position);
            self.command_tx
                .send(GameCommand::EngineCommands(commands))
                .unwrap();
        }
    }

    fn draw_all(&mut self, game_state: &GameState) {
        for x in 0..game_state.world.width() {
            for y in 0..game_state.world.height() {
                let position = v2(x, y);
                if let Some(cell) = game_state.world.get_cell(&position) {
                    if cell.object == WorldObject::House {
                        self.draw_house(&game_state, &v2(x, y));
                    }
                }
            }
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.house_artist = Some(HouseArtist::new(game_state.params.light_direction));
        self.draw_all(game_state);
    }
}

impl GameEventConsumer for HouseArtistHandler {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(&game_state),
            GameEvent::HouseUpdated { position, built } => {
                if *built {
                    self.draw_house(game_state, position);
                } else {
                    self.erase_house(game_state, position);
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
