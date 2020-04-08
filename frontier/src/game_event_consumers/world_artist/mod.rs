mod coloring;

pub use coloring::tile_color;
use coloring::*;

use super::*;
use isometric::Color;

const HANDLE: &str = "world_artist_handler";

pub struct WorldArtistHandler {
    command_tx: Sender<Vec<Command>>,
    world_artist: Option<WorldArtist>,
}

impl WorldArtistHandler {
    pub fn new(command_tx: Sender<Vec<Command>>) -> WorldArtistHandler {
        WorldArtistHandler {
            command_tx,
            world_artist: None,
        }
    }

    fn init(&mut self, game_state: &GameState) {
        let world_artist = WorldArtist::new(
            &game_state.world,
            WorldArtistParameters {
                road_color: Color::new(0.6, 0.4, 0.0, 1.0),
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
            let commands =
                world_artist.init(&game_state.world, &DefaultWorldColoring::new(game_state));
            self.command_tx.send(commands).unwrap();
        }
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        if let Some(ref mut world_artist) = self.world_artist {
            let commands = world_artist.draw_affected(
                &game_state.world,
                &DefaultWorldColoring::new(game_state),
                &cells,
            );
            self.command_tx.send(commands).unwrap();
        }
    }

    fn update_object(
        &mut self,
        game_state: &GameState,
        object: &WorldObject,
        position: &V2<usize>,
    ) {
        if let WorldObject::Farm = object {
            self.update_cells(game_state, &[*position]);
        }
    }

    fn draw_territory(&mut self, game_state: &GameState, changes: &[TerritoryChange]) {
        let affected: Vec<V2<usize>> = changes
            .iter()
            .flat_map(|change| game_state.world.expand_position(&change.position))
            .collect();
        self.update_cells(game_state, &affected);
    }
}

impl GameEventConsumer for WorldArtistHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

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
                object, position, ..
            } => self.update_object(game_state, object, position),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
