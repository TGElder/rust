use crate::actors::traits::Visibility;
use crate::game::Game;
use crate::polysender::Polysender;
use crate::road_builder::RoadBuilderResult;
use commons::async_trait::async_trait;
use commons::futures::future::FutureExt;
use std::sync::Arc;

#[async_trait]
pub trait UpdateRoads {
    async fn update_roads(&mut self, result: RoadBuilderResult);
}

#[async_trait]
impl UpdateRoads for Polysender {
    async fn update_roads(&mut self, result: RoadBuilderResult) {
        let result = Arc::new(result);
        let micros = send_update_world_get_micros(self, result.clone()).await;
        redraw(self, &result, micros);
        check_visibility_and_reveal(self, &result);
        update_pathfinder_with_roads(self, &result);
    }
}

async fn send_update_world_get_micros(tx: &mut Polysender, result: Arc<RoadBuilderResult>) -> u128 {
    tx.game
        .send(move |game| update_world_get_micros(game, result))
        .await
}

fn update_world_get_micros(game: &mut Game, result: Arc<RoadBuilderResult>) -> u128 {
    result.update_roads(&mut game.mut_state().world);
    game.game_state().game_micros
}

fn redraw(tx: &mut Polysender, result: &Arc<RoadBuilderResult>, micros: u128) {
    for position in result.path().iter().cloned() {
        tx.world_artist
            .send_future(move |artist| artist.redraw_tile_at(position, micros).boxed());
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
