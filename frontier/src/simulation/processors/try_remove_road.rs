use commons::edge::Edge;
use commons::log::trace;

use crate::traits::{RemoveRoad, SendWorld};

use super::*;

pub struct TryRemoveRoad<X> {
    x: X,
}

#[async_trait]
impl<X> Processor for TryRemoveRoad<X>
where
    X: RemoveRoad + SendWorld + Send + Sync + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let edges = match instruction {
            Instruction::RefreshEdges(edges) => edges.clone(),
            _ => return state,
        };

        let start = std::time::Instant::now();
        let mut count: usize = 0;
        let edge_count = edges.len();
        for edge in edges {
            if self.process_edge(&mut state, edge).await {
                count += 1;
            }
        }

        trace!(
            "Sent {}/{} build instructions in {}ms",
            count,
            edge_count,
            start.elapsed().as_millis()
        );

        state
    }
}

impl<X> TryRemoveRoad<X>
where
    X: RemoveRoad + SendWorld,
{
    pub fn new(x: X) -> TryRemoveRoad<X> {
        TryRemoveRoad { x }
    }

    async fn process_edge(&mut self, state: &mut State, edge: Edge) -> bool {
        if !state
            .edge_traffic
            .get(&edge)
            .map_or(true, |routes| routes.is_empty())
        {
            return false;
        }

        if self
            .x
            .send_world(move |world| {
                if !world.is_road(&edge) && world.road_planned(&edge).is_none() {
                    return true;
                }
                if world.road_planned(&edge).is_some() {
                    world.plan_road(&edge, None);
                }
                false
            })
            .await
        {
            return false;
        }

        state.build_queue.remove(&BuildKey::Road(edge));
        self.x.remove_road(&edge).await;

        true
    }
}
