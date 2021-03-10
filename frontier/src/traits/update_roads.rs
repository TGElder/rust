use crate::road_builder::RoadBuilderResult;
use crate::traits::{DrawWorld, Micros, UpdatePositionsAllPathfinders, Visibility, WithWorld};
use commons::async_trait::async_trait;
use commons::log::debug;
use commons::V2;
use std::collections::HashSet;
use std::iter::once;
use std::sync::Arc;

#[async_trait]
pub trait UpdateRoads {
    async fn update_roads(&self, result: RoadBuilderResult);
}

#[async_trait]
impl<T> UpdateRoads for T
where
    T: DrawWorld
        + Micros
        + UpdatePositionsAllPathfinders
        + Visibility
        + WithWorld
        + Send
        + Sync
        + 'static,
{
    async fn update_roads(&self, result: RoadBuilderResult) {
        // debug!("Updating road on {:?}", std::thread::current().name());
        let result = Arc::new(result);
        send_update_world(self, result.clone()).await;

        let edges = result.edges();
        let positions = once(edges[0].from())
            .chain(result.edges().iter().map(|edge| edge.to()))
            .copied()
            .collect::<HashSet<_>>();

        // debug!("Checking viz on {:?}", std::thread::current().name());
        check_visibility_and_reveal(self, positions.clone()).await;
        // debug!("Checked viz on {:?}", std::thread::current().name());

        // debug!("Finishing on {:?}", std::thread::current().name());
        join!(
            redraw(self, &positions),
            self.update_positions_all_pathfinders(positions.clone())
        );
        // debug!("Updated road on {:?}", std::thread::current().name());
    }
}

async fn send_update_world<T>(with_world: &T, result: Arc<RoadBuilderResult>)
where
    T: WithWorld,
{
    // debug!("Updating world");
    with_world
        .mut_world(|world| result.update_roads(world))
        .await;
    // debug!("Updated world");
}

async fn redraw<T>(cx: &T, positions: &HashSet<V2<usize>>)
where
    T: DrawWorld + Micros,
{
    // debug!("Redrawing");
    let micros = cx.micros().await;
    for position in positions {
        cx.draw_world_tile(*position, micros);
    }
    // debug!("Redrew");
}

async fn check_visibility_and_reveal<T>(cx: &T, positions: HashSet<V2<usize>>)
where
    T: Visibility + Send + Sync,
{
    cx.check_visibility_and_reveal(positions).await;
}
