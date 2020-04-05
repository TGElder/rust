use super::*;
use crate::travel_duration::TravelDuration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::default::Default;

const HANDLE: &str = "natural_road_sim";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NaturalRoadSimParams {
    visitor_count_threshold: usize,
}

impl Default for NaturalRoadSimParams {
    fn default() -> NaturalRoadSimParams {
        NaturalRoadSimParams {
            visitor_count_threshold: 8,
        }
    }
}

pub struct NaturalRoadSim {
    params: NaturalRoadSimParams,
    travel_duration: Arc<AutoRoadTravelDuration>,
    game_tx: UpdateSender<Game>,
}

impl Step for NaturalRoadSim {
    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl NaturalRoadSim {
    pub fn new(
        params: NaturalRoadSimParams,
        travel_duration: AutoRoadTravelDuration,
        game_tx: &UpdateSender<Game>,
    ) -> NaturalRoadSim {
        NaturalRoadSim {
            params,
            game_tx: game_tx.clone_with_handle(HANDLE),
            travel_duration: Arc::new(travel_duration),
        }
    }

    async fn step_async(&mut self) {
        let visitors = self.compute_visitors().await;
        let roads_to_build = self.roads_to_build(visitors);
        self.build_roads(roads_to_build).await;
    }

    async fn compute_visitors(&mut self) -> HashMap<Edge, usize> {
        let travel_duration = self.travel_duration.clone();
        self.game_tx
            .update(move |game| compute_visitors(game, travel_duration))
            .await
    }

    fn roads_to_build(&self, mut visitors: HashMap<Edge, usize>) -> Vec<Edge> {
        let threshold = self.params.visitor_count_threshold;
        visitors
            .drain()
            .filter(|(_, visitors)| *visitors >= threshold)
            .map(|(edge, _)| edge)
            .collect()
    }

    async fn build_roads(&mut self, mut roads: Vec<Edge>) {
        for road in roads.drain(..) {
            self.game_tx
                .update(move |game| build_road(game, road))
                .await
        }
    }
}

fn compute_visitors(
    game: &Game,
    travel_duration: Arc<AutoRoadTravelDuration>,
) -> HashMap<Edge, usize> {
    let mut out = HashMap::new();
    let game_state = game.game_state();
    for avatar in game_state.avatars.values() {
        if let Some(route) = &avatar.route {
            for edge in route.edges() {
                if should_compute_visitors(&game_state.world, &edge, &travel_duration) {
                    let visitors = out.entry(edge).or_insert(0);
                    *visitors += 1;
                }
            }
        }
    }
    out
}

fn should_compute_visitors(
    world: &World,
    edge: &Edge,
    travel_duration: &AutoRoadTravelDuration,
) -> bool {
    if world.is_road(&edge) {
        return false;
    }
    travel_duration
        .get_duration(&world, edge.from(), edge.to())
        .is_some()
}

fn build_road(game: &mut Game, road: Edge) {
    let road = vec![*road.from(), *road.to()];
    game.update_roads(RoadBuilderResult::new(road, true));
}
