use crate::avatar::AvatarTravelDuration;
use crate::polysender::Polysender;
use crate::road_builder::RoadBuilderResult;
use crate::traits::{Micros, Redraw, SendPathfinder, SendWorld, Visibility};
use commons::async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait UpdateRoads {
    async fn update_roads(&mut self, result: RoadBuilderResult);
}

#[async_trait]
impl UpdateRoads for Polysender
// where
//     T: Micros + Redraw + Visibility + WithPathfinder + WithWorld
{
    async fn update_roads(&mut self, result: RoadBuilderResult) {
        let result = Arc::new(result);
        send_update_world(self, result.clone()).await;
        let micros = self.micros().await;
        redraw(self, &result, micros);
        check_visibility_and_reveal(self, &result);
        update_pathfinder_with_roads(self, &result);
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

fn update_pathfinder_with_roads<W, P>(world: &mut W, pathfinder: &mut P, result: &Arc<RoadBuilderResult>) 
    where W: SendWorld, P: SendPathfinder<AvatarTravelDuration> + Send
{
        let result = result.clone();
        world.send_world_background(move |world| pathfinder.send_pathfinder_background(move |pathfinder| result.update_pathfinder(&world, &mut pathfinder)))
}
