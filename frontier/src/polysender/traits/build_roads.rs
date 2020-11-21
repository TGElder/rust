use commons::async_trait::async_trait;
use commons::edge::Edge;
use std::collections::HashSet;

use crate::game::Game;
use crate::polysender::traits::UpdateRoads;
use crate::polysender::Polysender;
use crate::road_builder::{RoadBuildMode, RoadBuilderResult};

#[async_trait]
pub trait BuildRoads {
    async fn add_road(&mut self, edge: &Edge);

    async fn remove_road(&mut self, edge: &Edge);
}

#[async_trait]
impl BuildRoads for Polysender {
    async fn add_road(&mut self, edge: &Edge) {
        if send_is_road(self, *edge).await {
            return;
        }
        let result = RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Build);
        self.update_roads(result).await;
    }

    async fn remove_road(&mut self, edge: &Edge) {
        if !send_is_road(self, *edge).await {
            return;
        }
        let result =
            RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Demolish);
        self.update_roads(result).await;
    }
}

pub async fn send_is_road(tx: &mut Polysender, edge: Edge) -> bool {
    tx.game.send(move |game| is_road(game, edge)).await
}

fn is_road(game: &mut Game, edge: Edge) -> bool {
    game.game_state().world.is_road(&edge)
}

#[async_trait]
impl BuildRoads for HashSet<Edge> {
    async fn add_road(&mut self, edge: &Edge) {
        self.insert(*edge);
    }

    async fn remove_road(&mut self, edge: &Edge) {
        self.remove(edge);
    }
}
