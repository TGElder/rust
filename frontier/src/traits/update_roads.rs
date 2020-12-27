use crate::road_builder::RoadBuilderResult;
use crate::traits::{DrawWorld, Micros, SendWorld, UpdatePositionsAllPathfinders, Visibility};
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
        + SendWorld
        + UpdatePositionsAllPathfinders
        + Visibility
        + Send
        + Sync
        + 'static,
{
    async fn update_roads(&self, result: RoadBuilderResult) {
        let result = Arc::new(result);
        send_update_world(self, result.clone()).await;

        check_visibility_and_reveal(self, &result);

        join!(
            redraw(self, &result),
            self.update_positions_all_pathfinders(result.path().clone())
        );
    }
}

async fn send_update_world<T>(send_world: &T, result: Arc<RoadBuilderResult>)
where
    T: SendWorld,
{
    send_world
        .send_world(move |world| result.update_roads(world))
        .await
}

async fn redraw<T>(tx: &T, result: &Arc<RoadBuilderResult>)
where
    T: DrawWorld + Micros,
{
    let micros = tx.micros().await;
    for position in result.path().iter().cloned() {
        tx.draw_world_tile(position, micros);
    }
}

fn check_visibility_and_reveal(tx: &dyn Visibility, result: &Arc<RoadBuilderResult>) {
    let visited = result.path().iter().cloned().collect();
    tx.check_visibility_and_reveal(visited);
}
