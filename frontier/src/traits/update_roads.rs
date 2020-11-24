use crate::avatar::AvatarTravelDuration;
use crate::pathfinder::Pathfinder;
use crate::polysender::Polysender;
use crate::road_builder::RoadBuilderResult;
use crate::traits::{
    Micros, PathfinderWithoutPlannedRoads, Redraw, SendGame, SendPathfinder, SendWorld, Visibility,
};
use crate::travel_duration::TravelDuration;
use commons::async_trait::async_trait;
use commons::V2;
use std::sync::Arc;
use std::time::Duration;

#[async_trait]
pub trait UpdateRoads {
    async fn update_roads(&mut self, result: RoadBuilderResult);
}

#[async_trait]
impl<T> UpdateRoads for T
where
    T: Micros + Redraw + Visibility + PathfinderWithoutPlannedRoads + SendWorld + Send + 'static,
{
    async fn update_roads(&mut self, result: RoadBuilderResult) {
        let result = Arc::new(result);
        send_update_world(self, result.clone()).await;
        let micros = self.micros().await;
        redraw(self, &result, micros);
        check_visibility_and_reveal(self, &result);

        let travel_duration = self.travel_duration_without_planned_roads().clone();
        let pathfinder = self.pathfinder_without_planned_roads().clone();

        update_pathfinder_with_roads(self, travel_duration, pathfinder, &result);
    }
}

async fn send_update_world<T>(send_world: &mut T, result: Arc<RoadBuilderResult>)
where
    T: SendWorld,
{
    send_world
        .send_world(move |world| result.update_roads(world))
        .await
}

fn redraw(redraw: &mut dyn Redraw, result: &Arc<RoadBuilderResult>, micros: u128) {
    for position in result.path().iter().cloned() {
        redraw.redraw_tile_at(position, micros);
    }
}

fn check_visibility_and_reveal(tx: &mut dyn Visibility, result: &Arc<RoadBuilderResult>) {
    let visited = result.path().iter().cloned().collect();
    tx.check_visibility_and_reveal(visited);
}

async fn update_pathfinder_with_roads<T, D, P>(
    tx: &mut T,
    travel_duration: Arc<D>,
    pathfinder: P,
    result: &Arc<RoadBuilderResult>,
) where
    T: SendWorld,
    D: TravelDuration + 'static,
    P: SendPathfinder<D> + Send,
{
    let path = result.path().clone();
    let durations: Vec<(V2<usize>, V2<usize>, Option<Duration>)> = tx
        .send_world(move |world| {
            (0..path.len() - 1)
                .flat_map(|i| {
                    let from = path[i];
                    let to = path[i + 1];
                    vec![
                        (from, to, travel_duration.get_duration(world, &from, &to)),
                        (to, from, travel_duration.get_duration(world, &to, &from)),
                    ]
                    .into_iter()
                })
                .collect()
        })
        .await;

    pathfinder.send_pathfinder_background(move |pathfinder| {
        for (from, to, duration) in durations {
            if let Some(duration) = duration {
                pathfinder.set_edge_duration(&from, &to, &duration)
            }
        }
    });
}
