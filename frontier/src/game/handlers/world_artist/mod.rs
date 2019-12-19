mod coloring;

use coloring::*;

use super::*;
use crate::world::*;
use commons::*;
use isometric::Color;

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
            WorldArtistParameters {
                road_color: Color::new(0.5, 0.5, 0.5, 1.0),
                river_color: Color::new(0.0, 0.0, 1.0, 1.0),
                waterfall_color: Color::new(0.0, 0.75, 1.0, 1.0),
                slab_size: 64,
                vegetation_exageration: 100.0,
                waterfall_gradient: game_state.params.avatar_travel.max_navigable_river_gradient,
            },
        );
        self.world_artist = Some(world_artist);
        self.draw_all(game_state);
    }

    fn draw_all(&mut self, game_state: &GameState) {
        if let Some(world_artist) = &mut self.world_artist {
            let command = GameCommand::EngineCommands(
                world_artist.init(&game_state.world, &create_coloring(game_state)),
            );
            self.command_tx.send(command).unwrap();
        }
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        if let Some(ref mut world_artist) = self.world_artist {
            let commands =
                world_artist.draw_affected(&game_state.world, &create_coloring(game_state), &cells);
            self.command_tx
                .send(GameCommand::EngineCommands(commands))
                .unwrap();
        }
    }

    fn draw_territory(&mut self, game_state: &GameState, changes: &[TerritoryChange]) {
        let affected: Vec<V2<usize>> = changes.iter().map(|change| change.position).collect();
        self.update_cells(game_state, &affected);
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
            GameEvent::TerritoryChanged(changes) => self.draw_territory(game_state, changes),
            GameEvent::ObjectUpdated {
                object: WorldObject::Farm,
                position,
                ..
            } => self.update_cells(game_state, &[*position]),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
