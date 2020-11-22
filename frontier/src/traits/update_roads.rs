use crate::polysender::Polysender;
use crate::road_builder::RoadBuilderResult;
use crate::traits::{Micros, Redraw, Visibility, WithWorld};
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

async fn send_update_world<W>(with_world: &mut W, result: Arc<RoadBuilderResult>)
where
    W: WithWorld,
{
    with_world
        .with_world(move |world| result.update_roads(world))
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

fn update_pathfinder_with_roads(tx: &mut Polysender, result: &Arc<RoadBuilderResult>) {
    for pathfinder in tx.pathfinders.iter().cloned() {
        let result = result.clone();
        tx.game.send(move |game| {
            result.update_pathfinder(&game.game_state().world, &mut pathfinder.write().unwrap())
        });
    }
}
