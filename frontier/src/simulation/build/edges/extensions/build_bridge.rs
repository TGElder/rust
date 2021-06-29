use std::collections::HashSet;
use std::convert::TryInto;
use std::time::Duration;

use commons::edge::Edge;

use crate::avatar::Vehicle;
use crate::bridge::{Bridge, BridgeType, Bridges};
use crate::build::{Build, BuildInstruction, BuildKey};
use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{
    GetBuildInstruction, InsertBuildInstruction, WithBridges, WithEdgeTraffic, WithRoutes,
};
use crate::travel_duration::EdgeDuration;

impl<T, D> EdgeBuildSimulation<T, D>
where
    T: GetBuildInstruction
        + HasParameters
        + InsertBuildInstruction
        + WithBridges
        + WithRoutes
        + WithEdgeTraffic
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
        let duration =
            Duration::from_millis(self.cx.parameters().built_bridge_1_cell_duration_millis);

        self.cx
            .with_bridges(|bridges| get_candidates(bridges, edges, &duration))
            .await
    }

    async fn try_build_bridge(&self, bridge: Bridge, threshold: usize) {
        let routes = self.get_route_summaries(&bridge.total_edge()).await;

        if routes.iter().map(|route| route.traffic).sum::<usize>() < threshold {
            return;
        }

        let when = self.get_when(routes, threshold);

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
}

fn get_candidates(bridges: &Bridges, edges: &HashSet<Edge>, duration: &Duration) -> Vec<Bridge> {
    edges
        .iter()
        .flat_map(|edge| get_candidate(bridges, edge, duration))
        .collect()
}

fn get_candidate(bridges: &Bridges, edge: &Edge, duration: &Duration) -> Option<Bridge> {
    let edge_bridges = bridges.get(edge)?;
    edge_bridges
        .iter()
        .find(|bridge| *bridge.bridge_type() == BridgeType::Theoretical)?;

    let built = Bridge::new(
        vec![EdgeDuration {
            from: *edge.from(),
            to: *edge.to(),
            duration: Some(*duration * edge.length().try_into().unwrap()),
        }],
        Vehicle::None,
        BridgeType::Built,
    )
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

    use commons::async_trait::async_trait;
    use commons::{v2, M, V2};
    use futures::executor::block_on;

    use crate::parameters::Parameters;
    use crate::resource::Resource;
    use crate::route::{Route, RouteKey, Routes, RoutesExt};
    use crate::traffic::EdgeTraffic;

    use super::*;

    struct Cx {
        bridges: Mutex<Bridges>,
        build_instructions: Mutex<Vec<BuildInstruction>>,
        edge_traffic: Mutex<EdgeTraffic>,
        parameters: Parameters,
        routes: Mutex<Routes>,
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

    fn happy_path_edge() -> Edge {
        Edge::new(v2(1, 0), v2(1, 2))
    }

    fn happy_path_tx() -> Cx {
        let bridges = hashmap! {
            happy_path_edge() => hashset!{
                Bridge::new(
                    vec![
                        EdgeDuration{
                            from: v2(1, 0),
                            to: v2(1, 2),
                            duration: Some(Duration::from_micros(1)),
                        }
                    ],
                    Vehicle::None,
                    BridgeType::Theoretical
                ).unwrap()
            }
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
            built_bridge_1_cell_duration_millis: 11,
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

        Cx {
            bridges: Mutex::new(bridges),
            build_instructions: Mutex::default(),
            edge_traffic: Mutex::new(edge_traffic),
            parameters,
            routes: Mutex::new(routes),
        }
    }

    #[test]
    fn should_build_bridge_if_traffic_meets_threshold() {
        // Given
        let sim = EdgeBuildSimulation::new(happy_path_tx(), Arc::new(()));

        // When
        block_on(sim.build_bridge(&hashset! {happy_path_edge()}));

        // Then
        let expected_build_queue = vec![BuildInstruction {
            what: Build::Bridge(
                Bridge::new(
                    vec![EdgeDuration {
                        from: v2(1, 0),
                        to: v2(1, 2),
                        duration: Some(Duration::from_millis(11 * 2)),
                    }],
                    Vehicle::None,
                    BridgeType::Built,
                )
                .unwrap(),
            ),
            when: 11,
        }];
        assert_eq!(
            *sim.cx.build_instructions.lock().unwrap(),
            expected_build_queue
        );
    }
}

//     #[test]
//     fn should_not_build_if_no_traffic_entry() {
//         // Given
//         let mut cx = happy_path_tx();
//         cx.edge_traffic = Mutex::default();
//         let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

//         // When
//         block_on(sim.build_road(&hashset! {happy_path_edge()}));

//         // Then
//         assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
//     }

//     #[test]
//     fn should_not_build_if_traffic_below_threshold() {
//         // Given
//         let sim = EdgeBuildSimulation::new(happy_path_tx(), happy_path_travel_duration());

//         // When
//         block_on(sim.build_road(&hashset! {Edge::new(v2(0, 0), v2(1, 0))}));

//         // Then
//         assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
//     }

//     #[test]
//     fn should_not_build_if_road_already_exists() {
//         // Given
//         let cx = happy_path_tx();
//         {
//             let mut world = cx.world.lock().unwrap();
//             world.set_road(&happy_path_edge(), true);
//         }
//         let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

//         // When
//         block_on(sim.build_road(&hashset! {happy_path_edge()}));

//         // Then
//         assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
//     }

//     #[test]
//     fn should_not_build_if_road_planned_earlier() {
//         // Given
//         let mut cx = happy_path_tx();
//         cx.road_planned = Some(1);
//         let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

//         // When
//         block_on(sim.build_road(&hashset! {happy_path_edge()}));

//         // Then
//         assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
//     }

//     #[test]
//     fn should_build_if_road_planned_later() {
//         // Given
//         let mut cx = happy_path_tx();
//         cx.road_planned = Some(12);
//         let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

//         // When
//         block_on(sim.build_road(&hashset! {happy_path_edge()}));

//         // Then
//         let expected_build_queue = vec![BuildInstruction {
//             what: Build::Road(happy_path_edge()),
//             when: 11,
//         }];
//         assert_eq!(
//             *sim.cx.build_instructions.lock().unwrap(),
//             expected_build_queue
//         );
//         assert_eq!(
//             *sim.cx.planned_roads.lock().unwrap(),
//             vec![(Edge::new(v2(1, 0), v2(1, 1)), Some(11))]
//         );
//     }

//     #[test]
//     fn should_not_build_if_road_not_possible() {
//         let sim = EdgeBuildSimulation::new(
//             happy_path_tx(),
//             Arc::new(MockTravelDuration { duration: None }),
//         );

//         // When
//         block_on(sim.build_road(&hashset! {happy_path_edge()}));

//         // Then
//         assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
//     }

//     #[test]
//     fn should_not_build_for_non_existent_route() {
//         // Given
//         let mut cx = happy_path_tx();
//         cx.routes = Mutex::default();
//         let sim = EdgeBuildSimulation::new(cx, happy_path_travel_duration());

//         // When
//         block_on(sim.build_road(&hashset! {happy_path_edge()}));

//         // Then
//         assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
//     }
// }
