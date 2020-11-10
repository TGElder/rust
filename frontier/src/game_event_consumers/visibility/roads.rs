use crate::actors::Visibility;
use crate::game::*;
use commons::fn_sender::FnSender;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

const HANDLE: &str = "visibility_from_roads";

pub struct VisibilityFromRoads {
    tx: FnSender<Visibility>,
}

impl VisibilityFromRoads {
    pub fn new(tx: &FnSender<Visibility>) -> VisibilityFromRoads {
        VisibilityFromRoads {
            tx: tx.clone_with_name(HANDLE),
        }
    }

    fn visit(&mut self, visited: &[V2<usize>]) {
        let visited = visited.iter().cloned().collect();
        self.tx
            .send(|visibility| visibility.check_visibility_and_reveal(visited));
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
