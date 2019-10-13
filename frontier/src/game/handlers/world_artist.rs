use super::*;
use crate::world::*;
use commons::*;
use isometric::drawing::*;

pub struct WorldArtistHandler {
    command_tx: Sender<GameCommand>,
    world_artist: Option<WorldArtist>,
}

impl WorldArtistHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> WorldArtistHandler {
        WorldArtistHandler {
            command_tx,
            world_artist: None,
        }
    }

    fn init(&mut self, game_state: &GameState) {
        let world_artist = WorldArtist::new(
            &game_state.world,
            Self::create_coloring(
                &game_state.world,
                game_state.params.light_direction,
                game_state.params.world_gen.cliff_gradient,
                game_state.params.world_gen.beach_level,
            ),
            64,
        );
        self.world_artist = Some(world_artist);
        self.draw_all(game_state);
    }

    fn draw_all(&mut self, game_state: &GameState) {
        if let Some(world_artist) = &mut self.world_artist {
            let command = GameCommand::EngineCommands(world_artist.init(&game_state.world));
            self.command_tx.send(command).unwrap();
        }
    }

    fn create_coloring(
        world: &World,
        light_direction: V3<f32>,
        cliff_gradient: f32,
        beach_level: f32,
    ) -> LayerColoring<WorldCell> {
        let snow_temperature = 0.0;
        let mut out = LayerColoring::default();
        out.add_layer(
            "base".to_string(),
            Box::new(DefaultColoring::new(
                &world,
                beach_level,
                snow_temperature,
                cliff_gradient,
                light_direction,
            )),
            0,
        );
        out
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        if let Some(ref mut world_artist) = self.world_artist {
            let commands = world_artist.draw_affected(&game_state.world, &cells);
            self.command_tx
                .send(GameCommand::EngineCommands(commands))
                .unwrap();
        }
    }
}

impl GameEventConsumer for WorldArtistHandler {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::CellsRevealed(selection) => {
                match selection {
                    CellSelection::All => self.draw_all(game_state),
                    CellSelection::Some(cells) => self.update_cells(game_state, &cells),
                };
            }
            GameEvent::RoadsUpdated(result) => self.update_cells(game_state, result.path()),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
