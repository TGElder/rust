use std::collections::HashSet;

use commons::edge::Edge;

use crate::build::{Build, BuildInstruction};
use crate::route::{Route, RouteKey, Routes, RoutesExt};
use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{
    InsertBuildInstruction, PlanRoad, RoadPlanned, WithEdgeTraffic, WithRoutes, WithWorld,
};
use crate::travel::TravelDuration;
use crate::world::World;

impl<T, D> EdgeBuildSimulation<T, D>
where
    T: HasParameters
        + InsertBuildInstruction
        + PlanRoad
        + RoadPlanned
        + WithRoutes
        + WithEdgeTraffic
        + WithWorld
        + Send
        + Sync,
    D: TravelDuration,
{
    pub async fn build_road(&self, edges: &HashSet<Edge>) {
        let threshold = self.cx.parameters().simulation.road_build_threshold;

        for candidate in self.get_candidates(edges).await {
            self.build_road_on_edge(candidate, threshold).await;
        }
    }

    async fn get_candidates(&self, edges: &HashSet<Edge>) -> Vec<Edge> {
        self.cx
            .with_world(|world| get_candidates(world, self.travel_duration.as_ref(), edges))
            .await
    }

    async fn build_road_on_edge(&self, edge: Edge, threshold: usize) {
        let routes = self.get_route_summaries(&edge).await;

        if routes.iter().map(|route| route.traffic).sum::<usize>() < threshold {
            return;
        }

        let when = self.get_when(routes, threshold);

        if !self.try_plan_road(&edge, when).await {
            return;
        }

        self.cx
            .insert_build_instruction(BuildInstruction {
                what: Build::Road(edge),
                when,
            })
            .await;
    }

    async fn get_route_summaries(&self, edge: &Edge) -> Vec<RouteSummary> {
        let route_keys = self.get_edge_traffic(edge).await;
        if route_keys.is_empty() {
            return vec![];
        }

        self.cx
            .with_routes(|routes| get_route_summaries(routes, route_keys))
            .await
    }

    async fn get_edge_traffic(&self, edge: &Edge) -> HashSet<RouteKey> {
        self.cx
            .with_edge_traffic(|edge_traffic| edge_traffic.get(edge).cloned().unwrap_or_default())
            .await
    }

    fn get_when(&self, mut routes: Vec<RouteSummary>, threshold: usize) -> u128 {
        routes.sort_by_key(|route| route.first_visit);
        let mut traffic_cum = 0;
        for route in routes {
            traffic_cum += route.traffic;
            if traffic_cum >= threshold {
                return route.first_visit;
            }
        }
        panic!(
            "Total traffic {} does not exceed threshold for building road {}",
            traffic_cum, threshold
        );
    }

    async fn try_plan_road(&self, edge: &Edge, when: u128) -> bool {
        if matches!(self.cx.road_planned(edge).await, Some(existing) if existing <= when) {
            false
        } else {
            self.cx.plan_road(edge, Some(when)).await;
            true
        }
    }
}

fn get_candidates(
    world: &World,
    travel_duration: &dyn TravelDuration,
    edges: &HashSet<Edge>,
) -> Vec<Edge> {
    edges
        .iter()
        .filter(|edge| is_candidate(world, travel_duration, edge))
        .copied()
        .collect()
}

fn is_candidate(world: &World, travel_duration: &dyn TravelDuration, edge: &Edge) -> bool {
    !world.is_road(edge)
        && travel_duration
            .get_duration(world, edge.from(), edge.to())
            .is_some()
}

fn get_route_summaries(routes: &Routes, route_keys: HashSet<RouteKey>) -> Vec<RouteSummary> {
    route_keys
        .into_iter()
        .flat_map(|route_key| routes.get_route(&route_key))
        .map(|route| route.into())
        .collect()
}

struct RouteSummary {
    traffic: usize,
    first_visit: u128,
}

impl From<&Route> for RouteSummary {
    fn from(route: &Route) -> Self {
        RouteSummary {
            traffic: route.traffic,
            first_visit: route.start_micros + route.duration.as_micros(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use commons::async_trait::async_trait;
    use commons::{v2, M, V2};
    use futures::executor::block_on;

    use crate::parameters::Parameters;
    use crate::resource::Resource;
    use crate::route::{RouteKey, Routes, RoutesExt};
    use crate::traffic::EdgeTraffic;

    use super::*;

    struct Cx {
        build_instructions: Mutex<Vec<BuildInstruction>>,
        edge_traffic: Mutex<EdgeTraffic>,
        parameters: Parameters,
        planned_roads: Mutex<Vec<(Edge, Option<u128>)>>,
        road_planned: Option<u128>,
        routes: Mutex<Routes>,
        world: Mutex<World>,
    }

    impl HasParameters for Cx {
        fn parameters(&self) -> &Parameters {
            &self.parameters
        }
    }

    #[async_trait]
    impl InsertBuildInstruction for Cx {
        async fn insert_build_instruction(&self, build_instruction: BuildInstruction) {
            self.build_instructions
                .lock()
                .unwrap()
                .push(build_instruction)
        }
    }

    #[async_trait]
    impl RoadPlanned for Cx {
        async fn road_planned(&self, _: &Edge) -> Option<u128> {
            self.road_planned
        }
    }

    #[async_trait]
    impl PlanRoad for Cx {
        async fn plan_road(&self, edge: &Edge, when: Option<u128>) {
            self.planned_roads.lock().unwrap().push((*edge, when));
        }
    }

    #[async_trait]
    impl WithRoutes for Cx {
        async fn with_routes<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Routes) -> O + Send,
        {
            function(&self.routes.lock().unwrap())
        }

        async fn mut_routes<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Routes) -> O + Send,
        {
            function(&mut self.routes.lock().unwrap())
        }
    }

    #[async_trait]
    impl WithWorld for Cx {
        async fn with_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&World) -> O + Send,
        {
            function(&self.world.lock().unwrap())
        }

        async fn mut_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut World) -> O + Send,
        {
            function(&mut self.world.lock().unwrap())
        }
    }

    #[async_trait]
    impl WithEdgeTraffic for Cx {
        async fn with_edge_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&EdgeTraffic) -> O + Send,
        {
            function(&self.edge_traffic.lock().unwrap())
        }

        async fn mut_edge_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut EdgeTraffic) -> O + Send,
        {
            function(&mut self.edge_traffic.lock().unwrap())
        }
    }

    struct MockTravelDuration {
        duration: Option<Duration>,
    }

    impl TravelDuration for MockTravelDuration {
        fn get_duration(&self, _: &World, _: &V2<usize>, _: &V2<usize>) -> Option<Duration> {
            self.duration
        }

        fn min_duration(&self) -> Duration {
            Duration::from_millis(1000)
        }

        fn max_duration(&self) -> Duration {
            Duration::from_millis(2000)
        }
    }

    fn happy_path_edge() -> Edge {
        Edge::new(v2(1, 0), v2(1, 1))
    }

    fn happy_path_tx() -> Cx {
        let edge_traffic = hashmap! {
            Edge::new(v2(0, 0), v2(1, 0)) => hashset!{
                RouteKey{
                    settlement: v2(0, 0),
                    resource: Resource::Truffles,
                    destination: v2(1, 1),
                }
            },
            Edge::new(v2(2, 0), v2(1, 0)) => hashset!{
                RouteKey{
                    settlement: v2(2, 0),
                    resource: Resource::Truffles,
                    destination: v2(1, 1),
                }
            },
            Edge::new(v2(1, 0), v2(1, 1)) => hashset!{
                RouteKey{
                    settlement: v2(0, 0),
                    resource: Resource::Truffles,
                    destination: v2(1, 1),
                }, RouteKey{
                    settlement: v2(2, 0),
                    resource: Resource::Truffles,
                    destination: v2(1, 1),
                }
            },
        };

        let mut parameters = Parameters::default();
        parameters.simulation.road_build_threshold = 8;

        let mut routes = Routes::default();
        routes.insert_route(
            RouteKey {
                settlement: v2(0, 0),
                resource: Resource::Truffles,
                destination: v2(1, 1),
            },
            Route {
                path: vec![v2(0, 0), v2(1, 0), v2(1, 1)],
                start_micros: 1,
                duration: Duration::from_micros(10),
                traffic: 4,
            },
        );
        routes.insert_route(
            RouteKey {
                settlement: v2(2, 0),
                resource: Resource::Truffles,
                destination: v2(1, 1),
            },
            Route {
                path: vec![v2(2, 0), v2(1, 0), v2(1, 1)],
                start_micros: 2,
                duration: Duration::from_micros(7),
                traffic: 4,
            },
        );

        let world = World::new(M::from_element(3, 3, 1.0), 0.5);

        Cx {
            build_instructions: Mutex::default(),
            edge_traffic: Mutex::new(edge_traffic),
            parameters,
            planned_roads: Mutex::default(),
            road_planned: None,
            routes: Mutex::new(routes),
            world: Mutex::new(world),
        }
    }

    fn happy_path_travel_duration() -> Arc<MockTravelDuration> {
        Arc::new(MockTravelDuration {
            duration: Some(Duration::from_millis(1500)),
        })
    }

    #[test]
    fn should_build_road_if_traffic_meets_threshold() {
        // Given
        let sim = EdgeBuildSimulation::new(happy_path_tx(), happy_path_travel_duration());

        // When
        block_on(sim.build_road(&hashset! {happy_path_edge()}));

        // Then
        let expected_build_queue = vec![BuildInstruction {
            what: Build::Road(happy_path_edge()),
            when: 11,
        }];
        assert_eq!(
            *sim.cx.build_instructions.lock().unwrap(),
            expected_build_queue
        );

        assert_eq!(
            *sim.cx.planned_roads.lock().unwrap(),
            vec![(Edge::new(v2(1, 0), v2(1, 1)), Some(11))]
        );
    }

    #[test]
    fn should_not_build_if_no_traffic_entry() {
        // Given
        let mut cx = happy_path_tx();
        cx.edge_traffic = Mutex::default();
        let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

        // When
        block_on(sim.build_road(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_traffic_below_threshold() {
        // Given
        let sim = EdgeBuildSimulation::new(happy_path_tx(), happy_path_travel_duration());

        // When
        block_on(sim.build_road(&hashset! {Edge::new(v2(0, 0), v2(1, 0))}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_road_already_exists() {
        // Given
        let cx = happy_path_tx();
        {
            let mut world = cx.world.lock().unwrap();
            world.set_road(&happy_path_edge(), true);
        }
        let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

        // When
        block_on(sim.build_road(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_road_planned_earlier() {
        // Given
        let mut cx = happy_path_tx();
        cx.road_planned = Some(1);
        let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

        // When
        block_on(sim.build_road(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_build_if_road_planned_later() {
        // Given
        let mut cx = happy_path_tx();
        cx.road_planned = Some(12);
        let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

        // When
        block_on(sim.build_road(&hashset! {happy_path_edge()}));

        // Then
        let expected_build_queue = vec![BuildInstruction {
            what: Build::Road(happy_path_edge()),
            when: 11,
        }];
        assert_eq!(
            *sim.cx.build_instructions.lock().unwrap(),
            expected_build_queue
        );
        assert_eq!(
            *sim.cx.planned_roads.lock().unwrap(),
            vec![(Edge::new(v2(1, 0), v2(1, 1)), Some(11))]
        );
    }

    #[test]
    fn should_not_build_if_road_not_possible() {
        let sim = EdgeBuildSimulation::new(
            happy_path_tx(),
            Arc::new(MockTravelDuration { duration: None }),
        );

        // When
        block_on(sim.build_road(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_for_non_existent_route() {
        // Given
        let mut cx = happy_path_tx();
        cx.routes = Mutex::default();
        let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

        // When
        block_on(sim.build_road(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }
}
