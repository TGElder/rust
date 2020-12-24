use std::collections::HashSet;

use commons::edge::Edge;

use crate::game::traits::GetRoute;
use crate::route::{Route, RouteKey, Routes};
use crate::traits::{PathfinderWithPlannedRoads, SendRoutes, SendWorld, UpdatePathfinderPositions};
use crate::travel_duration::TravelDuration;
use crate::world::World;

use super::*;

pub struct BuildRoad<T, D> {
    tx: T,
    travel_duration: Arc<D>,
}

#[async_trait]
impl<T, D> Processor for BuildRoad<T, D>
where
    T: PathfinderWithPlannedRoads
        + SendRoutes
        + SendWorldProxy
        + UpdatePathfinderPositions
        + Send
        + Sync
        + 'static,
    D: TravelDuration + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let edges = match instruction {
            Instruction::RefreshEdges(edges) => edges.clone(),
            _ => return state,
        };

        for candidate in self.get_candidates(edges).await {
            self.process_edge(&mut state, candidate).await;
        }

        state
    }
}

impl<X, T> BuildRoad<X, T>
where
    X: PathfinderWithPlannedRoads + SendRoutes + SendWorldProxy + UpdatePathfinderPositions,
    T: TravelDuration + 'static,
{
    pub fn new(x: X, travel_duration: Arc<T>) -> BuildRoad<X, T> {
        BuildRoad {
            tx: x,
            travel_duration,
        }
    }

    async fn get_candidates(&self, edges: HashSet<Edge>) -> Vec<Edge> {
        let travel_duration = self.travel_duration.clone();
        self.tx
            .send_world_proxy(move |world| get_candidates(world, travel_duration, edges))
            .await
    }

    async fn process_edge(&mut self, state: &mut State, edge: Edge) {
        let routes = self.get_route_summaries(state, &edge).await;

        if routes.iter().map(|route| route.traffic).sum::<usize>()
            < state.params.road_build_threshold
        {
            return;
        }

        let first_visit = routes
            .into_iter()
            .map(|route| route.first_visit)
            .min()
            .unwrap();

        if self
            .plan_road_if_not_planned_earlier(edge, first_visit)
            .await
        {
            return;
        }

        let pathfinder = self.tx.pathfinder_with_planned_roads().clone();
        self.tx
            .update_pathfinder_positions(pathfinder, vec![*edge.from(), *edge.to()])
            .await;

        state.build_queue.insert(BuildInstruction {
            what: Build::Road(edge),
            when: first_visit,
        });
    }

    async fn get_route_summaries(&self, state: &State, edge: &Edge) -> Vec<RouteSummary> {
        let route_keys = unwrap_or!(state.edge_traffic.get(&edge), return vec![]).clone();
        if route_keys.is_empty() {
            return vec![];
        }

        self.tx
            .send_routes(move |routes| get_route_summaries(routes, route_keys))
            .await
    }

    async fn plan_road_if_not_planned_earlier(&self, edge: Edge, when: u128) -> bool {
        self.tx
            .send_world_proxy(move |world| plan_road_if_not_planned_earlier(world, edge, when))
            .await
    }
}

fn get_candidates(
    world: &World,
    travel_duration: Arc<dyn TravelDuration>,
    edges: HashSet<Edge>,
) -> Vec<Edge> {
    edges
        .into_iter()
        .filter(|edge| is_candidate(world, travel_duration.as_ref(), edge))
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

fn plan_road_if_not_planned_earlier(world: &mut World, edge: Edge, when: u128) -> bool {
    if world
        .road_planned(&edge)
        .map_or(false, |existing| existing <= when)
    {
        return true;
    }
    world.plan_road(&edge, Some(when));
    false
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

// Required to stop the auto-implementation of UpdatePathfinderPositions for SendWorld when testing
#[async_trait]
pub trait SendWorldProxy {
    async fn send_world_proxy<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static;
}

#[async_trait]
impl<T> SendWorldProxy for T
where
    T: SendWorld + Send + Sync,
{
    async fn send_world_proxy<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static,
    {
        self.send_world(function).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use commons::{v2, Arm, M};
    use futures::executor::block_on;

    use crate::pathfinder::Pathfinder;
    use crate::resource::Resource;
    use crate::route::{RouteKey, Routes, RoutesExt};
    use crate::traits::SendPathfinder;

    use super::*;

    struct Tx {
        pathfinder: MockPathfinder,
        routes: Arm<Routes>,
        update_pathfinder_positions: Arm<Vec<V2<usize>>>,
        world: Arm<World>,
    }

    impl PathfinderWithPlannedRoads for Tx {
        type T = MockPathfinder;

        fn pathfinder_with_planned_roads(&self) -> &Self::T {
            &self.pathfinder
        }
    }

    #[async_trait]
    impl SendRoutes for Tx {
        async fn send_routes<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut Routes) -> O + Send + 'static,
        {
            function(&mut self.routes.lock().unwrap())
        }
    }

    #[async_trait]
    impl SendWorldProxy for Tx {
        async fn send_world_proxy<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut World) -> O + Send + 'static,
        {
            function(&mut self.world.lock().unwrap())
        }
    }

    #[async_trait]
    impl UpdatePathfinderPositions for Tx {
        async fn update_pathfinder_positions<P, I>(&self, _: P, positions: I)
        where
            P: SendPathfinder + Send + Sync,
            I: IntoIterator<Item = V2<usize>> + Send + Sync + 'static,
        {
            self.update_pathfinder_positions
                .lock()
                .unwrap()
                .extend(positions.into_iter());
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

    #[derive(Clone)]
    struct MockPathfinder {}

    #[async_trait]
    impl SendPathfinder for MockPathfinder {
        type T = MockTravelDuration;

        async fn send_pathfinder<F, O>(&self, _: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut Pathfinder<Self::T>) -> O + Send + 'static,
        {
            panic!("Not expecting pathfinder to be called in this test!")
        }

        fn send_pathfinder_background<F, O>(&self, _: F)
        where
            O: Send + 'static,
            F: FnOnce(&mut Pathfinder<Self::T>) -> O + Send + 'static,
        {
            panic!("Not expecting pathfinder to be called in this test!")
        }
    }

    fn happy_path_edge() -> Edge {
        Edge::new(v2(1, 0), v2(1, 1))
    }

    fn happy_path_tx() -> Tx {
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

        let mut world = World::new(M::from_element(3, 3, 1.0), 0.5);
        world.plan_road(&happy_path_edge(), Some(10));

        Tx {
            pathfinder: MockPathfinder {},
            routes: Arc::new(Mutex::new(routes)),
            update_pathfinder_positions: Arm::default(),
            world: Arc::new(Mutex::new(world)),
        }
    }

    fn happy_path_state() -> State {
        State {
            edge_traffic: hashmap! {
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
            },
            params: SimulationParams {
                road_build_threshold: 8,
                ..SimulationParams::default()
            },
            ..State::default()
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
        let mut processor = BuildRoad::new(happy_path_tx(), happy_path_travel_duration());

        // When
        let state = block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshEdges(hashset! {happy_path_edge()}),
        ));

        // Then
        let mut expected_build_queue = BuildQueue::default();
        expected_build_queue.insert(BuildInstruction {
            what: Build::Road(happy_path_edge()),
            when: 9,
        });
        assert_eq!(state.build_queue, expected_build_queue);

        assert_eq!(
            processor
                .tx
                .world
                .lock()
                .unwrap()
                .road_planned(&happy_path_edge()),
            Some(9)
        );

        assert_eq!(
            *processor.tx.update_pathfinder_positions.lock().unwrap(),
            vec![v2(1, 0), v2(1, 1)]
        );
    }

    #[test]
    fn should_not_build_if_no_traffic_entry() {
        // Given
        let mut state = happy_path_state();
        state.edge_traffic = hashmap! {};
        let mut processor = BuildRoad::new(happy_path_tx(), happy_path_travel_duration());

        // When
        let state = block_on(processor.process(
            state,
            &Instruction::RefreshEdges(hashset! {happy_path_edge()}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_if_traffic_below_threshold() {
        // Given
        let mut processor = BuildRoad::new(happy_path_tx(), happy_path_travel_duration());

        // When
        let state = block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshEdges(hashset! {Edge::new(v2(0, 0), v2(1, 0))}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_if_road_already_exists() {
        // Given
        let tx = happy_path_tx();
        {
            let mut world = tx.world.lock().unwrap();
            world.set_road(&happy_path_edge(), true);
        }
        let mut processor = BuildRoad::new(tx, happy_path_travel_duration());

        // When
        let state = block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshEdges(hashset! {happy_path_edge()}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_if_road_planned_earlier() {
        // Given
        let tx = happy_path_tx();
        {
            let mut world = tx.world.lock().unwrap();
            world.plan_road(&happy_path_edge(), Some(1));
        }
        let mut processor = BuildRoad::new(tx, happy_path_travel_duration());

        // When
        let state = block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshEdges(hashset! {happy_path_edge()}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_if_road_not_possible() {
        let mut processor = BuildRoad::new(
            happy_path_tx(),
            Arc::new(MockTravelDuration { duration: None }),
        );

        // When
        let state = block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshEdges(hashset! {happy_path_edge()}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_for_non_existent_route() {
        // Given
        let mut tx = happy_path_tx();
        tx.routes = Arm::default();
        let mut processor = BuildRoad::new(tx, happy_path_travel_duration());

        // When
        let state = block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshEdges(hashset! {happy_path_edge()}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }
}
