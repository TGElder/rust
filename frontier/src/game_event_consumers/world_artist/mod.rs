mod coloring;

pub use coloring::WorldColoringParameters;
use coloring::*;

use super::*;
use crate::artists::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "world_artist_handler";

struct WorldArtistHandlerBindings {
    toggle_territory_layer: Button,
}

pub struct WorldArtistHandler {
    command_tx: Sender<Vec<Command>>,
    bindings: WorldArtistHandlerBindings,
    world_artist: Option<WorldArtist>,
    territory_layer: bool,
}

impl WorldArtistHandler {
    pub fn new(command_tx: Sender<Vec<Command>>) -> WorldArtistHandler {
        WorldArtistHandler {
            command_tx,
            bindings: WorldArtistHandlerBindings {
                toggle_territory_layer: Button::Key(VirtualKeyCode::O),
            },
            world_artist: None,
            territory_layer: false,
        }
    }

    fn init(&mut self, game_state: &GameState) {
        let world_artist = WorldArtist::new(
            &game_state.world,
            WorldArtistParameters {
                waterfall_gradient: game_state.params.avatar_travel.max_navigable_river_gradient,
                ..WorldArtistParameters::default()
            },
        );
        self.world_artist = Some(world_artist);
        self.draw_all(game_state);
    }

    fn draw_all(&mut self, game_state: &GameState) {
        if let Some(world_artist) = &mut self.world_artist {
            let commands = world_artist.init(
                &game_state.world,
                &world_coloring(game_state, self.territory_layer),
            );
            self.command_tx.send(commands).unwrap();
        }
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        if let Some(ref mut world_artist) = self.world_artist {
            let commands = world_artist.draw_affected(
                &game_state.world,
                &world_coloring(game_state, self.territory_layer),
                &cells,
            );
            self.command_tx.send(commands).unwrap();
        }
    }

    fn draw_territory(&mut self, game_state: &GameState, changes: &[TerritoryChange]) {
        if !self.territory_layer {
            return;
        }
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
            GameEvent::ObjectUpdated(position) => self.update_cells(game_state, &[*position]),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.bindings.toggle_territory_layer {
                self.territory_layer = !self.territory_layer;
                self.draw_all(game_state);
            }
        }
        CaptureEvent::No
    }
}
