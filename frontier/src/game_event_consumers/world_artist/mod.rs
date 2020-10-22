use crate::actors::{Redraw, RedrawType};
use commons::async_channel::Sender as AsyncSender;
use commons::futures::executor::block_on;

use super::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "world_artist_handler";

struct WorldArtistHandlerBindings {
    toggle_territory_layer: Button,
}

pub struct WorldArtistHandler {
    actor_tx: AsyncSender<Redraw>,
    bindings: WorldArtistHandlerBindings,
    territory_layer: bool,
}

impl WorldArtistHandler {
    pub fn new(actor_tx: &AsyncSender<Redraw>) -> WorldArtistHandler {
        WorldArtistHandler {
            actor_tx: actor_tx.clone(),
            bindings: WorldArtistHandlerBindings {
                toggle_territory_layer: Button::Key(VirtualKeyCode::O),
            },
            territory_layer: true,
        }
    }

    fn draw_all(&mut self, game_state: &GameState) {
        block_on(self.actor_tx.send(Redraw {
            redraw_type: RedrawType::All,
            when: game_state.game_micros,
        }))
        .unwrap();
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        for cell in cells {
            block_on(self.actor_tx.send(Redraw {
                redraw_type: RedrawType::Tile(*cell),
                when: game_state.game_micros,
            }))
            .unwrap();
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
            GameEvent::CellsRevealed { selection, .. } => {
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
