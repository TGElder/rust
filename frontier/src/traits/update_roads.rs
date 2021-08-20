use crate::road_builder::RoadBuilderResult;
use crate::traits::{DrawWorld, UpdatePositionsAllPathfinders, Visibility, WithWorld};
use commons::async_trait::async_trait;
use commons::V2;
use std::collections::HashSet;
use std::sync::Arc;

#[async_trait]
pub trait UpdateRoads {
    async fn update_roads(&self, result: RoadBuilderResult);
}

#[async_trait]
impl<T> UpdateRoads for T
where
    T: DrawWorld + UpdatePositionsAllPathfinders + Visibility + WithWorld + Send + Sync + 'static,
{
    async fn update_roads(&self, result: RoadBuilderResult) {
        let result = Arc::new(result);
        send_update_world(self, result.clone()).await;

        let positions = result.positions();

        check_visibility_and_reveal(self, &positions).await;

        join!(
            self.draw_world_tiles(positions.clone()),
            self.update_positions_all_pathfinders(positions)
        );
    }
}

async fn send_update_world<T>(with_world: &T, result: Arc<RoadBuilderResult>)
where
    T: WithWorld,
{
    with_world
        .mut_world(|world| result.update_roads(world))
        .await;
}

async fn check_visibility_and_reveal<T>(cx: &T, positions: &HashSet<V2<usize>>)
where
    T: Visibility + Send + Sync,
{
    cx.check_visibility_and_reveal(positions).await;
}
