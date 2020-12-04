use crate::road_builder::RoadBuilderResult;
use crate::traits::{
    DrawWorld, Micros, PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendPathfinder,
    SendWorld, Visibility,
};
use crate::travel_duration::{EdgeDuration, TravelDuration};
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
        + Visibility
        + PathfinderWithoutPlannedRoads
        + PathfinderWithPlannedRoads
        + SendWorld
        + Send
        + Sync
        + 'static,
{
    async fn update_roads(&self, result: RoadBuilderResult) {
        let result = Arc::new(result);
        send_update_world(self, result.clone()).await;

        check_visibility_and_reveal(self, &result);

        join!(redraw(self, &result), update_pathfinders(self, &result));
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

async fn redraw<T>(x: &T, result: &Arc<RoadBuilderResult>)
where
    T: DrawWorld + Micros,
{
    let micros = x.micros().await;
    for position in result.path().iter().cloned() {
        x.draw_world_tile(position, micros);
    }
}

fn check_visibility_and_reveal(tx: &dyn Visibility, result: &Arc<RoadBuilderResult>) {
    let visited = result.path().iter().cloned().collect();
    tx.check_visibility_and_reveal(visited);
}

async fn update_pathfinders<T>(x: &T, result: &Arc<RoadBuilderResult>)
where
    T: PathfinderWithPlannedRoads + PathfinderWithoutPlannedRoads + SendWorld,
{
    let pathfinder_with = x.pathfinder_with_planned_roads().clone();
    let pathfinder_without = x.pathfinder_without_planned_roads().clone();

    join!(
        update_pathfinder(x, pathfinder_with, result),
        update_pathfinder(x, pathfinder_without, result),
    );
}

async fn update_pathfinder<T, P>(x: &T, pathfinder: P, result: &Arc<RoadBuilderResult>)
where
    T: SendWorld,
    P: SendPathfinder + Send,
{
    let travel_duration = pathfinder
        .send_pathfinder(|pathfinder| pathfinder.travel_duration().clone())
        .await;

    let path = result.path().clone();
    let durations: Vec<EdgeDuration> = x
        .send_world(move |world| {
            travel_duration
                .get_durations_for_path(world, &path)
                .collect()
        })
        .await;

    pathfinder.send_pathfinder_background(move |pathfinder| {
        for EdgeDuration { from, to, duration } in durations {
            if let Some(duration) = duration {
                pathfinder.set_edge_duration(&from, &to, &duration)
            }
        }
    });
}
