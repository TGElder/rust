use crate::actors::Visibility;
use crate::game::*;
use commons::actor::Actor;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

const HANDLE: &str = "visibility_from_roads";

pub struct VisibilityFromRoads<D>
where
    D: Actor<Visibility>,
{
    tx: D,
}

impl<D> VisibilityFromRoads<D>
where
    D: Actor<Visibility> + Clone,
{
    pub fn new(tx: &D) -> VisibilityFromRoads<D> {
        VisibilityFromRoads { tx: tx.clone() }
    }

    fn visit(&mut self, visited: &[V2<usize>]) {
        let visited = visited.iter().cloned().collect();
        self.tx
            .act(|visibility| visibility.check_visibility_and_reveal(visited));
    }
}

impl<D> GameEventConsumer for VisibilityFromRoads<D>
where
    D: Actor<Visibility> + Clone + Send,
{
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
