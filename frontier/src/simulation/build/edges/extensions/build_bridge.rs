use std::collections::{HashSet, VecDeque};

use commons::edge::Edge;
use commons::grid::Grid;

use crate::bridges::{Bridge, BridgeType, Bridges};
use crate::build::{Build, BuildInstruction, BuildKey};
use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{
    GetBuildInstruction, InsertBuildInstruction, WithBridges, WithEdgeTraffic, WithRoutes,
    WithWorld,
};

impl<T, D> EdgeBuildSimulation<T, D>
where
    T: GetBuildInstruction
        + HasParameters
        + InsertBuildInstruction
        + WithBridges
        + WithRoutes
        + WithEdgeTraffic
        + WithWorld
        + Send
        + Sync,
{
    pub async fn build_bridge(&self, edges: &HashSet<Edge>) {
        let threshold = self.cx.parameters().simulation.road_build_threshold;

        for candidate in self.get_bridge_candidates(edges).await {
            self.try_build_bridge(candidate, threshold).await;
        }
    }

    async fn get_bridge_candidates(&self, edges: &HashSet<Edge>) -> Vec<Bridge> {
        self.cx
            .with_bridges(|bridges| get_candidates(bridges, edges))
            .await
    }

    async fn try_build_bridge(&self, bridge: Bridge, threshold: usize) {
        let routes = self.get_route_summaries(&bridge.total_edge()).await;

        if routes.iter().map(|route| route.traffic).sum::<usize>() < threshold {
            return;
        }

        let when = self.get_when(routes, threshold);

        let bridge = self.raise_deck(bridge).await;

        if let Some(instruction) = self
            .cx
            .get_build_instruction(&BuildKey::Bridge(bridge.clone()))
            .await
        {
            if instruction.when <= when {
                return;
            }
        }

        self.cx
            .insert_build_instruction(BuildInstruction {
                what: Build::Bridge(bridge),
                when,
            })
            .await;
    }

    async fn raise_deck(&self, mut bridge: Bridge) -> Bridge {
        let deck_height = self.cx.parameters().bridge_deck_height;
        self.cx
            .with_world(move |world| {
                let mut piers = bridge.piers.iter_mut().collect::<VecDeque<_>>();
                piers.pop_front();
                piers.pop_back();
                for mut pier in piers {
                    let cell = world.get_cell_unsafe(&pier.position);

                    if cell.elevation <= world.sea_level() {
                        pier.elevation = world.sea_level() + deck_height;
                    } else if cell.river.here() {
                        pier.elevation = cell.elevation + deck_height;
                    }
                }
                bridge
            })
            .await
    }
}

fn get_candidates(bridges: &Bridges, edges: &HashSet<Edge>) -> Vec<Bridge> {
    edges
        .iter()
        .flat_map(|edge| get_candidate(bridges, edge))
        .collect()
}

fn get_candidate(bridges: &Bridges, edge: &Edge) -> Option<Bridge> {
    let edge_bridges = bridges.get(edge)?;
    let theoretical = edge_bridges
        .iter()
        .find(|bridge| bridge.bridge_type == BridgeType::Theoretical)?;

    let built = Bridge {
        bridge_type: BridgeType::Built,
        ..theoretical.clone()
    }
    .validate()
    .ok()?;

    if edge_bridges.contains(&built) {
        None
    } else {
        Some(built)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use commons::almost::Almost;
    use commons::async_trait::async_trait;
    use commons::{v2, M};
    use futures::executor::block_on;

    use crate::avatar::Vehicle;
    use crate::bridges::Pier;
    use crate::parameters::Parameters;
    use crate::resource::Resource;
    use crate::route::{Route, RouteKey, Routes, RoutesExt};
    use crate::traffic::EdgeTraffic;
    use crate::world::World;

    use super::*;

    struct Cx {
        bridges: Mutex<Bridges>,
        build_instructions: Mutex<Vec<BuildInstruction>>,
        edge_traffic: Mutex<EdgeTraffic>,
        parameters: Parameters,
        routes: Mutex<Routes>,
        world: Mutex<World>,
    }

    impl HasParameters for Cx {
        fn parameters(&self) -> &Parameters {
            &self.parameters
        }
    }

    #[async_trait]
    impl GetBuildInstruction for Cx {
        async fn get_build_instruction(&self, key: &BuildKey) -> Option<BuildInstruction> {
            self.build_instructions
                .lock()
                .unwrap()
                .iter()
                .find(|instruction| instruction.what.key() == *key)
                .cloned()
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
    impl WithBridges for Cx {
        async fn with_bridges<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Bridges) -> O + Send,
        {
            function(&self.bridges.lock().unwrap())
        }

        async fn mut_bridges<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Bridges) -> O + Send,
        {
            function(&mut self.bridges.lock().unwrap())
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

    fn happy_path_edge() -> Edge {
        Edge::new(v2(1, 0), v2(1, 2))
    }

    fn bridge(bridge_type: BridgeType) -> Bridge {
        Bridge {
            piers: vec![
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                },
                Pier {
                    position: v2(1, 1),
                    elevation: 1.5,
                    platform: false,
                },
                Pier {
                    position: v2(1, 2),
                    elevation: 2.0,
                    platform: false,
                },
            ],
            vehicle: Vehicle::None,
            bridge_type,
        }
    }

    fn happy_path_cx() -> Cx {
        let bridges = hashmap! {
            happy_path_edge() => hashset!{bridge(BridgeType::Theoretical)}
        };

        let edge_traffic = hashmap! {
            Edge::new(v2(0, 0), v2(1, 0)) => hashset!{
                RouteKey{
                    settlement: v2(0, 0),
                    resource: Resource::Truffles,
                    destination: v2(1, 2),
                }
            },
            Edge::new(v2(2, 0), v2(1, 0)) => hashset!{
                RouteKey{
                    settlement: v2(2, 0),
                    resource: Resource::Truffles,
                    destination: v2(1, 2),
                }
            },
            Edge::new(v2(1, 0), v2(1, 2)) => hashset!{
                RouteKey{
                    settlement: v2(0, 0),
                    resource: Resource::Truffles,
                    destination: v2(1, 2),
                }, RouteKey{
                    settlement: v2(2, 0),
                    resource: Resource::Truffles,
                    destination: v2(1, 2),
                }
            },
        };

        let mut parameters = Parameters {
            ..Parameters::default()
        };
        parameters.simulation.road_build_threshold = 8;

        let mut routes = Routes::default();
        routes.insert_route(
            RouteKey {
                settlement: v2(0, 0),
                resource: Resource::Truffles,
                destination: v2(1, 2),
            },
            Route {
                path: vec![v2(0, 0), v2(1, 0), v2(1, 2)],
                start_micros: 1,
                duration: Duration::from_micros(10),
                traffic: 4,
            },
        );
        routes.insert_route(
            RouteKey {
                settlement: v2(2, 0),
                resource: Resource::Truffles,
                destination: v2(1, 2),
            },
            Route {
                path: vec![v2(2, 0), v2(1, 0), v2(1, 2)],
                start_micros: 2,
                duration: Duration::from_micros(7),
                traffic: 4,
            },
        );

        let world = World::new(M::from_element(3, 3, 1.0), 0.5);

        Cx {
            bridges: Mutex::new(bridges),
            build_instructions: Mutex::default(),
            edge_traffic: Mutex::new(edge_traffic),
            parameters,
            routes: Mutex::new(routes),
            world: Mutex::new(world),
        }
    }

    #[test]
    fn should_build_bridge_if_traffic_meets_threshold() {
        // Given
        let sim = EdgeBuildSimulation::new(happy_path_cx(), Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        let expected_build_queue = vec![BuildInstruction {
            what: Build::Bridge(bridge(BridgeType::Built)),
            when: 11,
        }];
        assert_eq!(
            *sim.cx.build_instructions.lock().unwrap(),
            expected_build_queue
        );
    }

    #[test]
    fn should_not_build_if_no_traffic_entry() {
        // Given
        let mut cx = happy_path_cx();
        cx.edge_traffic = Mutex::default();
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_traffic_below_threshold() {
        // Given
        let mut cx = happy_path_cx();
        cx.parameters.simulation.road_build_threshold = 9;
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_no_theoretical_bridge() {
        // Given
        let cx = happy_path_cx();
        cx.bridges
            .lock()
            .unwrap()
            .get_mut(&happy_path_edge())
            .unwrap()
            .clear();
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_bridge_already_exists() {
        // Given
        let cx = happy_path_cx();
        cx.bridges
            .lock()
            .unwrap()
            .get_mut(&happy_path_edge())
            .unwrap()
            .insert(bridge(BridgeType::Built));
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_bridge_planned_earlier() {
        // Given
        let cx = happy_path_cx();
        let earlier = BuildInstruction {
            what: Build::Bridge(bridge(BridgeType::Built)),
            when: 10,
        };
        cx.build_instructions.lock().unwrap().push(earlier.clone());
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        assert_eq!(*sim.cx.build_instructions.lock().unwrap(), vec![earlier]);
    }

    #[test]
    fn should_build_if_bridge_planned_later() {
        // Given
        let cx = happy_path_cx();
        cx.build_instructions
            .lock()
            .unwrap()
            .push(BuildInstruction {
                what: Build::Bridge(bridge(BridgeType::Built)),
                when: 12,
            });
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        assert!(sim
            .cx
            .build_instructions
            .lock()
            .unwrap()
            .contains(&BuildInstruction {
                what: Build::Bridge(bridge(BridgeType::Built)),
                when: 11,
            }));
    }

    #[test]
    fn should_raise_deck_over_sea() {
        // Given
        let mut cx = happy_path_cx();

        cx.parameters.bridge_deck_height = 0.45;

        cx.world
            .lock()
            .unwrap()
            .mut_cell_unsafe(&v2(1, 1))
            .elevation = 0.0;
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        let build = &sim.cx.build_instructions.lock().unwrap()[0].what;
        match build {
            Build::Bridge(bridge) => {
                assert!(bridge.piers[1].elevation.almost(&0.95));
            }
            _ => panic!("Expecting bridge build, found {:?}", build),
        }
    }

    #[test]
    fn should_raise_deck_over_river() {
        // Given
        let mut cx = happy_path_cx();

        cx.parameters.bridge_deck_height = 0.45;

        cx.world
            .lock()
            .unwrap()
            .mut_cell_unsafe(&v2(1, 1))
            .river
            .horizontal
            .width = 1.0;
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        let build = &sim.cx.build_instructions.lock().unwrap()[0].what;
        match build {
            Build::Bridge(bridge) => {
                assert!(bridge.piers[1].elevation.almost(&1.45));
            }
            _ => panic!("Expecting bridge build, found {:?}", build),
        }
    }

    #[test]
    fn should_not_raise_first_or_last_piers() {
        // Given
        let mut cx = happy_path_cx();

        cx.parameters.bridge_deck_height = 0.45;

        cx.world
            .lock()
            .unwrap()
            .mut_cell_unsafe(&v2(1, 1))
            .elevation = 0.0;
        cx.world
            .lock()
            .unwrap()
            .mut_cell_unsafe(&v2(1, 2))
            .elevation = 0.0;
        let sim = EdgeBuildSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        let build = &sim.cx.build_instructions.lock().unwrap()[0].what;
        match build {
            Build::Bridge(bridge) => {
                assert!(bridge.piers[0].elevation.almost(&1.0));
                assert!(bridge.piers[2].elevation.almost(&2.0));
            }
            _ => panic!("Expecting bridge build, found {:?}", build),
        }
    }
}
