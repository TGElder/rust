use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::grid::Grid;
use commons::V2;

use crate::build::{Build, BuildInstruction};
use crate::route::{RouteKey, RoutesExt};
use crate::settlement::{Settlement, SettlementClass};
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{
    AnyoneControls, GetSettlement, InsertBuildInstruction, RandomTownName, WithRouteToPorts,
    WithRoutes, WithTraffic, WithWorld,
};

impl<T> PositionBuildSimulation<T>
where
    T: AnyoneControls
        + GetSettlement
        + HasParameters
        + InsertBuildInstruction
        + RandomTownName
        + WithRoutes
        + WithRouteToPorts
        + WithTraffic
        + WithWorld,
{
    pub async fn build_town(&self, positions: HashSet<V2<usize>>) {
        for position in positions {
            self.build_town_at_position(position).await
        }
    }

    async fn build_town_at_position(&self, position: V2<usize>) {
        let cliff_gradient = &self.cx.parameters().world_gen.cliff_gradient;

        let (route_keys, anyone_controls_position, tiles) = join!(
            self.get_route_keys(&position),
            self.cx.anyone_controls(&position),
            self.get_tiles(&position, cliff_gradient)
        );

        if route_keys.is_empty() || anyone_controls_position || tiles.is_empty() {
            return;
        }

        let routes = self.get_route_summaries(route_keys).await;
        if routes.is_empty() || routes.iter().map(|route| route.traffic).sum::<usize>() == 0 {
            return;
        }
        let route = first_visit_route(routes);

        let settlement = unwrap_or!(self.cx.get_settlement(&route.settlement).await, return);
        let nation = settlement.nation;
        let name = ok_or!(self.cx.random_town_name(&nation).await, return);
        let initial_town_population = self.cx.parameters().simulation.initial_town_population;

        for tile in tiles {
            let settlement = Settlement {
                class: SettlementClass::Town,
                position: tile,
                name: name.clone(),
                nation: nation.clone(),
                current_population: initial_town_population,
                target_population: initial_town_population,
                gap_half_life: Duration::from_millis(0),
                last_population_update_micros: route.first_visit,
            };

            self.cx
                .insert_build_instruction(BuildInstruction {
                    what: Build::Town(settlement),
                    when: route.first_visit,
                })
                .await;
        }
    }

    async fn get_route_keys(&self, position: &V2<usize>) -> Vec<RouteKey> {
        let traffic = self.get_traffic(position).await;
        let route_to_ports = self.get_route_to_ports(&traffic).await;
        traffic
            .iter()
            .filter(|route| {
                route.destination == *position
                    || route_to_ports
                        .get(route)
                        .map_or(false, |ports| ports.contains(&position))
            })
            .cloned()
            .collect()
    }

    async fn get_traffic(&self, position: &V2<usize>) -> HashSet<RouteKey> {
        self.cx
            .with_traffic(|traffic| traffic.get(position).map(|traffic| traffic.clone()))
            .await
            .unwrap_or(hashset! {})
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    async fn get_route_to_ports<'a>(
        &self,
        route_keys: &'a HashSet<RouteKey>,
    ) -> HashMap<&'a RouteKey, HashSet<V2<usize>>> {
        self.cx
            .with_route_to_ports(|route_to_ports| {
                route_keys
                    .iter()
                    .map(|route_key| {
                        (
                            route_key,
                            route_to_ports.get(route_key).cloned().unwrap_or_default(),
                        )
                    })
                    .collect()
            })
            .await
    }

    async fn get_tiles(&self, position: &V2<usize>, cliff_gradient: &f32) -> Vec<V2<usize>> {
        self.cx
            .with_world(|world| {
                world
                    .get_adjacent_tiles_in_bounds(position)
                    .into_iter()
                    .filter(|tile| {
                        world.get_cell(tile).map_or(false, |cell| cell.visible)
                            && !world.is_sea(tile)
                            && world.get_max_abs_rise(tile) < *cliff_gradient
                    })
                    .collect()
            })
            .await
    }

    async fn get_route_summaries(&self, route_keys: Vec<RouteKey>) -> Vec<RouteSummary> {
        self.cx
            .with_routes(|routes| {
                route_keys
                    .into_iter()
                    .flat_map(|route_key| {
                        routes.get_route(&route_key).map(|route| RouteSummary {
                            settlement: route_key.settlement,
                            traffic: route.traffic,
                            first_visit: route.start_micros + route.duration.as_micros(),
                        })
                    })
                    .collect()
            })
            .await
    }
}

fn first_visit_route(routes: Vec<RouteSummary>) -> RouteSummary {
    routes
        .into_iter()
        .min_by_key(|route| route.first_visit)
        .unwrap()
}

struct RouteSummary {
    settlement: V2<usize>,
    traffic: usize,
    first_visit: u128,
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;

    use commons::async_trait::async_trait;
    use commons::index2d::Vec2D;
    use commons::{v2, M};
    use futures::executor::block_on;

    use crate::build::BuildKey;
    use crate::parameters::Parameters;
    use crate::resource::Resource;
    use crate::route::{Route, Routes, RoutesExt};
    use crate::traffic::Traffic;
    use crate::traits::NationNotFound;
    use crate::world::World;

    use super::*;

    struct Cx {
        anyone_controls: bool,
        build_instructions: Mutex<HashMap<BuildKey, BuildInstruction>>,
        get_settlement: Option<Settlement>,
        parameters: Parameters,
        random_town_name: String,
        route_to_ports: Mutex<HashMap<RouteKey, HashSet<V2<usize>>>>,
        routes: Mutex<Routes>,
        traffic: Mutex<Traffic>,
        world: Mutex<World>,
    }

    impl Default for Cx {
        fn default() -> Self {
            let mut world = World::new(M::from_element(3, 3, 1.0), 0.0);
            world.reveal_all();
            Cx {
                anyone_controls: false,
                build_instructions: Mutex::default(),
                get_settlement: None,
                parameters: Parameters::default(),
                random_town_name: String::default(),
                route_to_ports: Mutex::default(),
                routes: Mutex::default(),
                traffic: Mutex::new(Traffic::same_size_as(&world, hashset! {})),
                world: Mutex::new(world),
            }
        }
    }

    #[async_trait]
    impl AnyoneControls for Cx {
        async fn anyone_controls(&self, _: &V2<usize>) -> bool {
            self.anyone_controls
        }
    }

    #[async_trait]
    impl GetSettlement for Cx {
        async fn get_settlement(&self, _: &V2<usize>) -> Option<Settlement> {
            self.get_settlement.clone()
        }
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
                .insert(build_instruction.what.key(), build_instruction);
        }
    }

    #[async_trait]
    impl RandomTownName for Cx {
        async fn random_town_name(&self, _: &str) -> Result<String, NationNotFound> {
            Ok(self.random_town_name.clone())
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
    impl WithRouteToPorts for Cx {
        async fn with_route_to_ports<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send,
        {
            function(&self.route_to_ports.lock().unwrap())
        }

        async fn mut_route_to_ports<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send,
        {
            function(&mut self.route_to_ports.lock().unwrap())
        }
    }

    #[async_trait]
    impl WithTraffic for Cx {
        async fn with_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Traffic) -> O + Send,
        {
            function(&self.traffic.lock().unwrap())
        }

        async fn mut_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Traffic) -> O + Send,
        {
            function(&mut self.traffic.lock().unwrap())
        }
    }

    fn happy_path_route_key() -> RouteKey {
        RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Iron,
            destination: v2(1, 1),
        }
    }

    fn happy_path_tx() -> Cx {
        let cx = Cx {
            get_settlement: Some(Settlement {
                position: v2(0, 0),
                nation: "nation".to_string(),
                ..Settlement::default()
            }),
            random_town_name: "town".to_string(),
            ..Cx::default()
        };

        cx.routes.lock().unwrap().insert_route(
            happy_path_route_key(),
            Route {
                path: vec![v2(0, 0), v2(1, 0), v2(1, 1)],
                start_micros: 1,
                duration: Duration::from_micros(10),
                traffic: 1,
            },
        );

        {
            let mut traffic = cx.traffic.lock().unwrap();
            traffic
                .get_mut(&v2(0, 0))
                .unwrap()
                .insert(happy_path_route_key());
            traffic
                .get_mut(&v2(1, 0))
                .unwrap()
                .insert(happy_path_route_key());
            traffic
                .get_mut(&v2(1, 1))
                .unwrap()
                .insert(happy_path_route_key());
        }

        cx
    }

    #[test]
    fn should_build_if_route_ends_at_position() {
        // Given
        let mut cx = happy_path_tx();
        cx.parameters.simulation.initial_town_population = 1.1;
        let build_town = PositionBuildSimulation::new(cx);

        // When
        block_on(build_town.build_town(hashset! {v2(1, 1)}));

        // Then
        let build_instructions = build_town.cx.build_instructions.lock().unwrap();
        assert_eq!(
            *build_instructions.get(&BuildKey::Town(v2(1, 1))).unwrap(),
            BuildInstruction {
                what: Build::Town(Settlement {
                    class: SettlementClass::Town,
                    position: v2(1, 1),
                    name: "town".to_string(),
                    nation: "nation".to_string(),
                    current_population: 1.1,
                    target_population: 1.1,
                    gap_half_life: Duration::from_millis(0),
                    last_population_update_micros: 11,
                }),
                when: 11
            }
        );
        assert!(build_instructions.get(&BuildKey::Town(v2(0, 0))).is_some());
        assert!(build_instructions.get(&BuildKey::Town(v2(1, 0))).is_some());
        assert!(build_instructions.get(&BuildKey::Town(v2(0, 1))).is_some());
    }

    #[test]
    fn should_not_build_for_any_route() {
        // Given
        let sim = PositionBuildSimulation::new(happy_path_tx());

        // When
        block_on(sim.build_town(hashset! {v2(1, 0)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_build_if_port_at_position() {
        // Given
        let cx = happy_path_tx();
        cx.route_to_ports
            .lock()
            .unwrap()
            .insert(happy_path_route_key(), hashset! {v2(1, 0)});

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 0)}));

        // Then
        let build_instructions = sim.cx.build_instructions.lock().unwrap();
        assert!(build_instructions.get(&BuildKey::Town(v2(0, 0))).is_some());
        assert!(build_instructions.get(&BuildKey::Town(v2(1, 0))).is_some());
    }

    #[test]
    fn should_not_build_if_no_traffic_entry() {
        // Given
        let cx = happy_path_tx();
        *cx.traffic.lock().unwrap() = Vec2D::new(3, 3, HashSet::new());

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_zero_traffic() {
        // Given
        let cx = happy_path_tx();
        cx.routes.lock().unwrap().insert_route(
            happy_path_route_key(),
            Route {
                path: vec![v2(0, 0), v2(1, 0), v2(1, 1)],
                start_micros: 1,
                duration: Duration::from_micros(10),
                traffic: 0,
            },
        );

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_tile_invisible() {
        // Given
        let cx = happy_path_tx();
        {
            let mut world = cx.world.lock().unwrap();
            world.mut_cell_unsafe(&v2(0, 0)).visible = false;
            world.mut_cell_unsafe(&v2(1, 0)).visible = false;
            world.mut_cell_unsafe(&v2(0, 1)).visible = false;
            world.mut_cell_unsafe(&v2(1, 1)).visible = false;
        }

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_in_sea() {
        // Given
        let cx = happy_path_tx();
        *cx.world.lock().unwrap() = World::new(M::zeros(3, 3), 0.5);

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_on_cliff() {
        // Given
        let mut cx = happy_path_tx();
        cx.parameters.world_gen.cliff_gradient = 1.0;
        cx.world
            .lock()
            .unwrap()
            .mut_cell_unsafe(&v2(1, 1))
            .elevation = 2.0;

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_position_controlled() {
        // Given
        let mut cx = happy_path_tx();
        cx.anyone_controls = true;

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_for_non_existent_route() {
        // Given
        let cx = happy_path_tx();
        *cx.routes.lock().unwrap() = Routes::default();

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_for_non_existent_settlement() {
        // Given
        let mut cx = happy_path_tx();
        cx.get_settlement = None;

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_town(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }
}
