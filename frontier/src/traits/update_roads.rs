use crate::road_builder::RoadBuilderResult;
use crate::traits::{
    Micros, PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, Redraw, SendPathfinder,
    SendWorld, Visibility,
};
use crate::travel_duration::{PathDuration, TravelDuration};
use commons::async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait UpdateRoads {
    async fn update_roads(&self, result: RoadBuilderResult);
}

#[async_trait]
impl<T> UpdateRoads for T
where
    T: Micros
        + Redraw
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
        let micros = self.micros().await;
        redraw(self, &result, micros);
        check_visibility_and_reveal(self, &result);

        let pathfinder_with = self.pathfinder_with_planned_roads().clone();
        let pathfinder_without = self.pathfinder_without_planned_roads().clone();

        join!(
            update_path_durations(self, pathfinder_with, &result),
            update_path_durations(self, pathfinder_without, &result),
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

fn redraw(redraw: &dyn Redraw, result: &Arc<RoadBuilderResult>, micros: u128) {
    for position in result.path().iter().cloned() {
        redraw.redraw_tile_at(position, micros);
    }
}

fn check_visibility_and_reveal(tx: &dyn Visibility, result: &Arc<RoadBuilderResult>) {
    let visited = result.path().iter().cloned().collect();
    tx.check_visibility_and_reveal(visited);
}

async fn update_path_durations<T, P>(tx: &T, pathfinder: P, result: &Arc<RoadBuilderResult>)
where
    T: SendWorld,
    P: SendPathfinder + Send,
{
    let travel_duration = pathfinder
        .send_pathfinder(|pathfinder| pathfinder.travel_duration().clone())
        .await;

    let path = result.path().clone();
    let durations: Vec<PathDuration> = tx
        .send_world(move |world| travel_duration.get_path_durations(world, &path).collect())
        .await;

    pathfinder.send_pathfinder_background(move |pathfinder| {
        for PathDuration { from, to, duration } in durations {
            if let Some(duration) = duration {
                pathfinder.set_edge_duration(&from, &to, &duration)
            }
        }
    });
}
