use crate::game::*;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

use crate::game_event_consumers::VisibilityHandlerMessage;
use std::sync::mpsc::Sender;

const HANDLE: &str = "visibility_from_roads";

pub struct VisibilityFromRoads {
    tx: Sender<VisibilityHandlerMessage>,
}

impl VisibilityFromRoads {
    pub fn new(tx: &Sender<VisibilityHandlerMessage>) -> VisibilityFromRoads {
        VisibilityFromRoads { tx: tx.clone() }
    }

    fn visit(&mut self, visited: &[V2<usize>]) {
        self.tx
            .send(VisibilityHandlerMessage {
                visited: visited.iter().cloned().collect(),
            })
            .unwrap();
    }
}

impl GameEventConsumer for VisibilityFromRoads {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::RoadsUpdated(result) = event {
            self.visit(result.path());
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
