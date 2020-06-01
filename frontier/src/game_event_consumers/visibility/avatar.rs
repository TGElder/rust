use crate::avatar::*;
use crate::game::*;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

use crate::game_event_consumers::VisibilityHandlerMessage;
use std::iter::{empty, once};
use std::sync::mpsc::Sender;

const HANDLE: &str = "visibility_from_avatar";

pub struct VisibilityFromAvatar {
    tx: Sender<VisibilityHandlerMessage>,
}

impl VisibilityFromAvatar {
    pub fn new(tx: &Sender<VisibilityHandlerMessage>) -> VisibilityFromAvatar {
        VisibilityFromAvatar { tx: tx.clone() }
    }

    fn tick(&mut self, game_state: &GameState, from: &u128, to: &u128) {
        self.tx
            .send(VisibilityHandlerMessage {
                visited: avatar_visited_cells(game_state, from, to).collect(),
            })
            .unwrap();
    }
}

fn avatar_visited_cells<'a>(
    game_state: &'a GameState,
    from: &'a u128,
    to: &'a u128,
) -> Box<dyn Iterator<Item = V2<usize>> + 'a> {
    if let Some(avatar) = game_state.selected_avatar() {
        match &avatar.state {
            AvatarState::Walking(path) => {
                let edges = path.edges_between_times(from, to);
                return Box::new(edges.into_iter().map(|edge| *edge.to()));
            }
            AvatarState::Stationary { position, .. } => return Box::new(once(*position)),
            _ => (),
        }
    }
    Box::new(empty())
}

impl GameEventConsumer for VisibilityFromAvatar {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Tick {
            from_micros,
            to_micros,
        } = event
        {
            self.tick(game_state, from_micros, to_micros);
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
