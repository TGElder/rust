use commons::edge::Edge;
use commons::log::info;

use crate::game::traits::GetRoute;
use crate::route::Route;
use crate::traits::{PathfinderWithPlannedRoads, SendGame, SendPathfinder, SendWorld};
use crate::travel_duration::{EdgeDuration, TravelDuration};

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
        for edge in edges {
            if self.process_edge(&mut state, edge).await {
                count += 1;
            }
        }

        info!(
            "Sent {}/{} build instructions in {}ms",
            count,
            edge_count,
            start.elapsed().as_millis()
        );

        state
    }
}

impl<X, T> TryBuildRoad<X, T>
where
    X: PathfinderWithPlannedRoads + SendGame + SendWorld,
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

        let travel_duration_in_world = self.travel_duration.clone();
        if self
            .x
            .send_world(move |world| {
                if world.is_road(&edge)
                    || world
                        .road_planned(&edge)
                        .map_or(false, |when| when <= first_visit)
                {
                    return true;
                }
                if travel_duration_in_world
                    .get_duration(world, edge.from(), edge.to())
                    .is_none()
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

        self.update_pathfinder(&edge).await;

        state.build_queue.insert(BuildInstruction {
            what: Build::Road(edge),
            when: first_visit,
        });

        true
    }

    async fn update_pathfinder(&self, edge: &Edge) {
        let travel_duration = self
            .x
            .pathfinder_with_planned_roads()
            .send_pathfinder(|pathfinder| pathfinder.travel_duration().clone())
            .await;

        let path = [*edge.from(), *edge.to()];
        let durations: Vec<EdgeDuration> = self
            .x
            .send_world(move |world| {
                travel_duration
                    .get_durations_for_path(world, &path)
                    .collect()
            })
            .await;

        self.x
            .pathfinder_with_planned_roads()
            .send_pathfinder_background(move |pathfinder| {
                for EdgeDuration { from, to, duration } in durations {
                    if let Some(duration) = duration {
                        pathfinder.set_edge_duration(&from, &to, &duration)
                    }
                }
            });
    }
}
