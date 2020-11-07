use crate::actors::Visibility;
use crate::game::*;
use commons::async_update::UpdateSender;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

const HANDLE: &str = "visibility_from_roads";

pub struct VisibilityFromRoads {
    tx: UpdateSender<Visibility>,
}

impl VisibilityFromRoads {
    pub fn new(tx: &UpdateSender<Visibility>) -> VisibilityFromRoads {
        VisibilityFromRoads { tx: tx.clone() }
    }

    fn visit(&mut self, visited: &[V2<usize>]) {
        let visited = visited.iter().cloned().collect();
        self.tx
            .update(|visibility| visibility.check_visibility_and_reveal(visited));
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
