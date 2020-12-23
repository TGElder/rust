use std::collections::HashSet;

use commons::edge::Edge;
use commons::log::trace;

use crate::game::traits::GetRoute;
use crate::route::Route;
use crate::traits::{PathfinderWithPlannedRoads, SendGame, SendWorld, UpdatePathfinderPositions};
use crate::travel_duration::TravelDuration;
use crate::world::World;

use super::*;

const ROAD_THRESHOLD: usize = 8;

pub struct TryBuildRoad<X, T> {
    x: X,
    travel_duration: Arc<T>,
}

#[async_trait]
impl<X, T> Processor for TryBuildRoad<X, T>
where
    X: PathfinderWithPlannedRoads + SendGame + SendWorld + Send + Sync + 'static,
    T: TravelDuration + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let edges = match instruction {
            Instruction::RefreshEdges(edges) => edges.clone(),
            _ => return state,
        };

        let start = std::time::Instant::now();
        let mut count: usize = 0;
        let edge_count = edges.len();

        let travel_duration = self.travel_duration.clone();
        let candidates = self
            .x
            .send_world(move |world| candidates(world, travel_duration, edges))
            .await;
        let candidate_count = candidates.len();

        for candidate in candidates {
            if self.process_edge(&mut state, candidate).await {
                count += 1;
            }
        }

        trace!(
            "Sent {}/{}/{} build instructions in {}ms",
            count,
            candidate_count,
            edge_count,
            start.elapsed().as_millis()
        );

        state
    }
}

impl<X, T> TryBuildRoad<X, T>
where
    X: PathfinderWithPlannedRoads + SendGame + SendWorld + UpdatePathfinderPositions,
    T: TravelDuration + 'static,
{
    pub fn new(x: X, travel_duration: Arc<T>) -> TryBuildRoad<X, T> {
        TryBuildRoad { x, travel_duration }
    }

    async fn process_edge(&mut self, state: &mut State, edge: Edge) -> bool {
        let routes = unwrap_or!(state.edge_traffic.get(&edge), return false).clone();
        if routes.is_empty() {
            return false;
        }

        let routes: Vec<Route> = self
            .x
            .send_game(move |game| {
                routes
                    .into_iter()
                    .flat_map(|route_key| game.game_state().routes.get_route(&route_key))
                    .cloned()
                    .collect()
            })
            .await;

        if routes.iter().map(|route| route.traffic).sum::<usize>() < ROAD_THRESHOLD {
            return false;
        }

        let first_visit = routes
            .into_iter()
            .map(|route| route.start_micros + route.duration.as_micros())
            .min()
            .unwrap();

        if self
            .x
            .send_world(move |world| {
                if world
                    .road_planned(&edge)
                    .map_or(false, |when| when <= first_visit)
                {
                    return true;
                }
                world.plan_road(&edge, Some(first_visit));
                false
            })
            .await
        {
            return false;
        }

        let pathfinder = self.x.pathfinder_with_planned_roads().clone();
        self.x
            .update_pathfinder_positions(pathfinder, vec![*edge.from(), *edge.to()])
            .await;

        state.build_queue.insert(BuildInstruction {
            what: Build::Road(edge),
            when: first_visit,
        });

        true
    }
}

fn candidates(
    world: &World,
    travel_duration: Arc<dyn TravelDuration>,
    edges: HashSet<Edge>,
) -> Vec<Edge> {
    edges
        .into_iter()
        .filter(|edge| {
            !world.is_road(edge)
                && travel_duration
                    .get_duration(world, edge.from(), edge.to())
                    .is_some()
        })
        .collect()
}
