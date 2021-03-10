use crate::road_builder::RoadBuilderResult;
use crate::traits::{DrawWorld, Micros, UpdatePositionsAllPathfinders, Visibility, WithWorld};
use commons::async_trait::async_trait;
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
        let result = Arc::new(result);
        send_update_world(self, result.clone()).await;

        check_visibility_and_reveal(self, &result).await;

        join!(
            redraw(self, &result),
            self.update_positions_all_pathfinders(result.path().clone())
        );
    }
}

async fn send_update_world<T>(with_world: &T, result: Arc<RoadBuilderResult>)
where
    T: WithWorld,
{
    with_world
        .mut_world(|world| result.update_roads(world))
        .await
}

async fn redraw<T>(cx: &T, result: &Arc<RoadBuilderResult>)
where
    T: DrawWorld + Micros,
{
    let micros = cx.micros().await;
    for position in result.path().iter().cloned() {
        cx.draw_world_tile(position, micros);
    }
}

async fn check_visibility_and_reveal<T>(cx: &T, result: &Arc<RoadBuilderResult>)
where
    T: Visibility + Send + Sync,
{
    let visited = result.path().iter().cloned().collect();
    cx.check_visibility_and_reveal(visited).await;
}
