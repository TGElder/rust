use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::grid::Grid;

use crate::build::{Build, BuildInstruction};
use crate::route::{RouteKey, RoutesExt};
use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{
    AnyoneControls, GetSettlement, InsertBuildInstruction, RandomTownName, SendRoutes, SendWorld,
    WithRouteToPorts, WithTraffic,
};

use super::*;
pub struct BuildTown<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for BuildTown<T>
where
    T: AnyoneControls
        + GetSettlement
        + InsertBuildInstruction
        + RandomTownName
        + SendRoutes
        + SendWorld
        + WithRouteToPorts
        + WithTraffic
        + Send
        + Sync
        + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };

        for position in positions {
            self.process_position(&mut state, position).await
        }

        state
    }
}

impl<T> BuildTown<T>
where
    T: AnyoneControls
        + GetSettlement
        + InsertBuildInstruction
        + RandomTownName
        + SendRoutes
        + SendWorld
        + WithRouteToPorts
        + WithTraffic,
{
    pub fn new(tx: T) -> BuildTown<T> {
        BuildTown { tx }
    }

    async fn process_position(&mut self, state: &mut State, position: V2<usize>) {
        let (route_keys, anyone_controls_position, tiles) = join!(
            self.get_route_keys(&position),
            self.tx.anyone_controls(position),
            self.get_tiles(position)
        );

        if route_keys.is_empty() || anyone_controls_position || tiles.is_empty() {
            return;
        }

        let routes = self.get_route_summaries(route_keys).await;
        if routes.is_empty() || routes.iter().map(|route| route.traffic).sum::<usize>() == 0 {
            return;
        }
        let route = first_visit_route(routes);

        let settlement = unwrap_or!(self.tx.get_settlement(route.settlement).await, return);
        let nation = settlement.nation;
        let name = ok_or!(self.tx.random_town_name(nation.clone()).await, return);

        for tile in tiles {
            let settlement = Settlement {
                class: SettlementClass::Town,
                position: tile,
                name: name.clone(),
                nation: nation.clone(),
                current_population: state.params.initial_town_population,
                target_population: state.params.initial_town_population,
                gap_half_life: Duration::from_millis(0),
                last_population_update_micros: route.first_visit,
            };

            self.tx
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
        self.tx
            .with_traffic(|traffic| traffic.get(position).map(|traffic| traffic.clone()))
            .await
            .unwrap_or(hashset! {})
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    async fn get_route_to_ports<'a>(
        &self,
        route_keys: &'a HashSet<RouteKey>,
    ) -> HashMap<&'a RouteKey, HashSet<V2<usize>>> {
        self.tx
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

    async fn get_tiles(&self, position: V2<usize>) -> Vec<V2<usize>> {
        self.tx
            .send_world(move |world| {
                world
                    .get_adjacent_tiles_in_bounds(&position)
                    .into_iter()
                    .filter(|tile| {
                        world.get_cell(tile).map_or(false, |cell| cell.visible)
                            && !world.is_sea(tile)
                    })
                    .collect()
            })
            .await
    }

    async fn get_route_summaries(&self, route_keys: Vec<RouteKey>) -> Vec<RouteSummary> {
        self.tx
            .send_routes(move |routes| {
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

    use commons::index2d::Vec2D;
    use commons::{v2, M};
    use futures::executor::block_on;

    use crate::build::BuildKey;
    use crate::resource::Resource;
    use crate::route::{Route, Routes, RoutesExt};
    use crate::traits::NationNotFound;
    use crate::world::World;

    use super::*;

    struct Tx {
        anyone_controls: bool,
        build_instructions: Mutex<HashMap<BuildKey, BuildInstruction>>,
        get_settlement: Option<Settlement>,
        random_town_name: String,
        routes: Mutex<Routes>,
        route_to_ports: Mutex<HashMap<RouteKey, HashSet<V2<usize>>>>,
        traffic: Mutex<Traffic>,
        world: Mutex<World>,
    }

    impl Default for Tx {
        fn default() -> Self {
            let mut world = World::new(M::from_element(3, 3, 1.0), 0.0);
            world.reveal_all();
            Tx {
                anyone_controls: false,
                build_instructions: Mutex::default(),
                get_settlement: None,
                random_town_name: String::default(),
                routes: Mutex::default(),
                route_to_ports: Mutex::default(),
                traffic: Mutex::new(Traffic::same_size_as(&world, hashset! {})),
                world: Mutex::new(world),
            }
        }
    }

    #[async_trait]
    impl AnyoneControls for Tx {
        async fn anyone_controls(&self, _: V2<usize>) -> bool {
            self.anyone_controls
        }
    }

    #[async_trait]
    impl GetSettlement for Tx {
        async fn get_settlement(&self, _: V2<usize>) -> Option<Settlement> {
            self.get_settlement.clone()
        }
    }

    #[async_trait]
    impl InsertBuildInstruction for Tx {
        async fn insert_build_instruction(&self, build_instruction: BuildInstruction) {
            self.build_instructions
                .lock()
                .unwrap()
                .insert(build_instruction.what.key(), build_instruction);
        }
    }

    #[async_trait]
    impl RandomTownName for Tx {
        async fn random_town_name(&self, _: String) -> Result<String, NationNotFound> {
            Ok(self.random_town_name.clone())
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
    impl SendWorld for Tx {
        async fn send_world<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut World) -> O + Send + 'static,
        {
            function(&mut self.world.lock().unwrap())
        }

        fn send_world_background<F, O>(&self, function: F)
        where
            O: Send + 'static,
            F: FnOnce(&mut World) -> O + Send + 'static,
        {
            function(&mut self.world.lock().unwrap());
        }
    }

    #[async_trait]
    impl WithRouteToPorts for Tx {
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
    impl WithTraffic for Tx {
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

    fn happy_path_tx() -> Tx {
        let tx = Tx {
            get_settlement: Some(Settlement {
                position: v2(0, 0),
                nation: "nation".to_string(),
                ..Settlement::default()
            }),
            random_town_name: "town".to_string(),
            ..Tx::default()
        };

        tx.routes.lock().unwrap().insert_route(
            happy_path_route_key(),
            Route {
                path: vec![v2(0, 0), v2(1, 0), v2(1, 1)],
                start_micros: 1,
                duration: Duration::from_micros(10),
                traffic: 1,
            },
        );

        {
            let mut traffic = tx.traffic.lock().unwrap();
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

        tx
    }

    fn happy_path_state() -> State {
        let mut state = State {
            traffic: Vec2D::new(3, 3, HashSet::new()),
            ..State::default()
        };
        state.params.initial_town_population = 1.1;
        state
    }

    #[test]
    fn should_build_if_route_ends_at_position() {
        // Given
        let mut processor = BuildTown::new(happy_path_tx());

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        let build_instructions = processor.tx.build_instructions.lock().unwrap();
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
        let state = happy_path_state();
        let mut processor = BuildTown::new(happy_path_tx());

        // When
        block_on(processor.process(state, &Instruction::RefreshPositions(hashset! {v2(1, 0)})));

        // Then
        assert!(processor.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_build_if_port_at_position() {
        // Given
        let tx = happy_path_tx();
        tx.route_to_ports
            .lock()
            .unwrap()
            .insert(happy_path_route_key(), hashset! {v2(1, 0)});

        let state = happy_path_state();

        let mut processor = BuildTown::new(tx);

        // When
        block_on(processor.process(state, &Instruction::RefreshPositions(hashset! {v2(1, 0)})));

        // Then
        let build_instructions = processor.tx.build_instructions.lock().unwrap();
        assert!(build_instructions.get(&BuildKey::Town(v2(0, 0))).is_some());
        assert!(build_instructions.get(&BuildKey::Town(v2(1, 0))).is_some());
    }

    #[test]
    fn should_not_build_if_no_traffic_entry() {
        // Given
        let tx = happy_path_tx();
        *tx.traffic.lock().unwrap() = Vec2D::new(3, 3, HashSet::new());

        let mut processor = BuildTown::new(tx);

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert!(processor.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_zero_traffic() {
        // Given
        let tx = happy_path_tx();
        tx.routes.lock().unwrap().insert_route(
            happy_path_route_key(),
            Route {
                path: vec![v2(0, 0), v2(1, 0), v2(1, 1)],
                start_micros: 1,
                duration: Duration::from_micros(10),
                traffic: 0,
            },
        );

        let mut processor = BuildTown::new(tx);

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert!(processor.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_tile_invisible() {
        // Given
        let tx = happy_path_tx();
        {
            let mut world = tx.world.lock().unwrap();
            world.mut_cell_unsafe(&v2(0, 0)).visible = false;
            world.mut_cell_unsafe(&v2(1, 0)).visible = false;
            world.mut_cell_unsafe(&v2(0, 1)).visible = false;
            world.mut_cell_unsafe(&v2(1, 1)).visible = false;
        }

        let mut processor = BuildTown::new(tx);

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert!(processor.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_in_sea() {
        // Given
        let tx = happy_path_tx();
        *tx.world.lock().unwrap() = World::new(M::zeros(3, 3), 0.5);

        let mut processor = BuildTown::new(tx);

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert!(processor.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_if_position_controlled() {
        // Given
        let mut tx = happy_path_tx();
        tx.anyone_controls = true;

        let mut processor = BuildTown::new(tx);

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert!(processor.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_for_non_existent_route() {
        // Given
        let tx = happy_path_tx();
        *tx.routes.lock().unwrap() = Routes::default();

        let mut processor = BuildTown::new(tx);

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert!(processor.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_for_non_existent_settlement() {
        // Given
        let mut tx = happy_path_tx();
        tx.get_settlement = None;

        let mut processor = BuildTown::new(tx);

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert!(processor.tx.build_instructions.lock().unwrap().is_empty());
    }
}
