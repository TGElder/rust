use super::*;
use crate::travel_duration::TravelDuration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::default::Default;

const HANDLE: &str = "natural_road_sim";
const BATCH_SIZE: usize = 128;

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
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {}

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

    async fn get_routes(&mut self) -> Vec<String> {
        self.game_tx.update(|game| get_routes(game)).await
    }

    async fn compute_visitors(&mut self) -> HashMap<Edge, usize> {
        let mut out = HashMap::new();
        let routes = self.get_routes().await;
        for batch in routes.chunks(BATCH_SIZE) {
            for (edge, visitors) in self.compute_visitors_for_routes(batch.to_vec()).await {
                *out.entry(edge).or_insert(0) += visitors;
            }
        }
        out
    }

    async fn compute_visitors_for_routes(&mut self, routes: Vec<String>) -> HashMap<Edge, usize> {
        let travel_duration = self.travel_duration.clone();
        self.game_tx
            .update(move |game| compute_visitors_for_routes(game, travel_duration, routes))
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

fn get_routes(game: &Game) -> Vec<String> {
    game.game_state().routes.keys().cloned().collect()
}

fn compute_visitors_for_routes(
    game: &Game,
    travel_duration: Arc<AutoRoadTravelDuration>,
    routes: Vec<String>,
) -> HashMap<Edge, usize> {
    let game_state = game.game_state();
    routes
        .iter()
        .flat_map(|route| game_state.routes.get(route))
        .flat_map(|route| route.path.edges())
        .filter(|edge| should_compute_visitors(&game_state, &edge, &travel_duration))
        .fold(HashMap::new(), |mut map, edge| {
            *map.entry(edge).or_insert(0) += 1;
            map
        })
}

fn should_compute_visitors(
    game_state: &GameState,
    edge: &Edge,
    travel_duration: &AutoRoadTravelDuration,
) -> bool {
    let world = &game_state.world;
    if world.is_road(&edge) {
        return false;
    }
    if !visited(game_state, edge.from()) || !visited(game_state, edge.to()) {
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
