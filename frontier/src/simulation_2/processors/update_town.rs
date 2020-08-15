use super::*;
use crate::game::traits::{GetRoute, HasWorld, Settlements, UpdateSettlement};
use crate::route::RouteKey;
use crate::settlement::Settlement;
use commons::{get_corners, unsafe_ordering};
use std::collections::{HashMap, HashSet};

const HANDLE: &str = "update_town";

pub struct UpdateTown<G>
where
    G: HasWorld + GetRoute + Settlements + UpdateSettlement,
{
    game: UpdateSender<G>,
}

impl<G> Processor for UpdateTown<G>
where
    G: HasWorld + GetRoute + Settlements + UpdateSettlement,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let (settlement, territory) = match instruction {
            Instruction::UpdateTown {
                settlement,
                territory,
            } => (settlement, territory),
            _ => return state,
        };

        let route_keys = get_route_keys(territory, &state);

        let mut state =
            self.try_update_settlement(state, settlement.clone(), route_keys, territory.clone());

        state
            .instructions
            .push(Instruction::UpdateCurrentPopulation(settlement.position));

        state
    }
}

impl<G> UpdateTown<G>
where
    G: HasWorld + GetRoute + Settlements + UpdateSettlement,
{
    pub fn new(game: &UpdateSender<G>) -> UpdateTown<G> {
        UpdateTown {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn try_update_settlement(
        &mut self,
        state: State,
        settlement: Settlement,
        route_keys: HashSet<RouteKey>,
        territory: HashSet<V2<usize>>,
    ) -> State {
        block_on(async {
            self.game
                .update(move |game| {
                    try_update_settlement(game, state, settlement, route_keys, territory)
                })
                .await
        })
    }
}

fn get_route_keys(territory: &HashSet<V2<usize>>, state: &State) -> HashSet<RouteKey> {
    territory
        .iter()
        .flat_map(|position| state.traffic.get(position))
        .flatten()
        .cloned()
        .collect()
}

fn try_update_settlement<G>(
    game: &mut G,
    state: State,
    settlement: Settlement,
    route_keys: HashSet<RouteKey>,
    territory: HashSet<V2<usize>>,
) -> State
where
    G: HasWorld + GetRoute + Settlements + UpdateSettlement,
{
    let nation_to_traffic_share = get_nation_to_traffic_share(game, &state, route_keys, territory);

    let mut settlement = settlement;
    settlement.target_population =
        get_target_population(&nation_to_traffic_share, state.params.traffic_to_population);
    if let Some(new_nation) =
        get_new_nation(nation_to_traffic_share, state.params.nation_flip_traffic_pc)
    {
        settlement.nation = new_nation;
    }

    game.update_settlement(settlement);

    state
}

fn get_nation_to_traffic_share<G>(
    game: &mut G,
    state: &State,
    route_keys: HashSet<RouteKey>,
    territory: HashSet<V2<usize>>,
) -> HashMap<String, f64>
where
    G: HasWorld + GetRoute + Settlements,
{
    let mut out = hashmap! {};
    for route_key in route_keys {
        update_nation_to_traffic_share_for_route(game, state, route_key, &territory, &mut out);
    }
    out
}

fn update_nation_to_traffic_share_for_route<G>(
    game: &G,
    state: &State,
    route_key: RouteKey,
    territory: &HashSet<V2<usize>>,
    nation_to_traffic_share: &mut HashMap<String, f64>,
) where
    G: HasWorld + GetRoute + Settlements,
{
    if territory.contains(&route_key.settlement) {
        return;
    }
    let nation = &unwrap_or!(get_settlement(game, &route_key.settlement), return).nation;
    let route = unwrap_or!(game.get_route(&route_key), return);
    let ports = state
        .route_to_ports
        .get(&route_key)
        .cloned()
        .unwrap_or_default();
    let (ports_in_territory, ports_outside_territory): (Vec<V2<usize>>, Vec<V2<usize>>) =
        ports.into_iter().partition(|port| territory.contains(port));
    let denominator = (ports_in_territory.len() + ports_outside_territory.len() + 1) as f64;
    let is_destination = territory.contains(&route_key.destination) as usize;
    let multiplier = is_destination + ports_in_territory.len();
    let numerator = (route.traffic * multiplier) as f64;
    let traffic = numerator / denominator;

    *nation_to_traffic_share
        .entry(nation.to_string())
        .or_default() += traffic;
}

fn get_settlement<'a, G>(game: &'a G, position: &V2<usize>) -> Option<&'a Settlement>
where
    G: Settlements,
{
    game.settlements()
        .values()
        .find(|settlement| get_corners(&settlement.position).contains(position))
}

fn get_target_population(
    nation_to_traffic_share: &HashMap<String, f64>,
    traffic_to_population: f64,
) -> f64 {
    let total_traffic_share: f64 = nation_to_traffic_share.values().sum();
    total_traffic_share * traffic_to_population
}

fn get_new_nation(
    nation_to_traffic_share: HashMap<String, f64>,
    nation_flip_threshold: f64,
) -> Option<String> {
    let total_traffic_share: f64 = nation_to_traffic_share.values().sum();
    let (max_nation, max_traffic_share) = nation_to_traffic_share
        .into_iter()
        .max_by(|a, b| unsafe_ordering(&a.1, &b.1))?;
    if max_traffic_share / total_traffic_share >= nation_flip_threshold {
        Some(max_nation)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::route::Route;
    use crate::world::World;
    use commons::grid::Grid;
    use commons::update::UpdateProcess;
    use commons::{v2, M};

    use std::collections::{HashMap, HashSet};
    use std::default::Default;
    use std::time::Duration;

    struct MockGame {
        routes: HashMap<RouteKey, Route>,
        settlements: HashMap<V2<usize>, Settlement>,
        updated_settlements: Vec<Settlement>,
        world: World,
    }

    impl Default for MockGame {
        fn default() -> MockGame {
            MockGame {
                routes: hashmap! {},
                settlements: hashmap! {},
                updated_settlements: vec![],
                world: World::new(M::zeros(4, 4), -0.5),
            }
        }
    }

    impl HasWorld for MockGame {
        fn world(&self) -> &World {
            &self.world
        }

        fn world_mut(&mut self) -> &mut World {
            &mut self.world
        }
    }

    impl GetRoute for MockGame {
        fn get_route(&self, route_key: &RouteKey) -> Option<&Route> {
            self.routes.get(route_key)
        }
    }

    impl Settlements for MockGame {
        fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
            &self.settlements
        }
    }

    impl UpdateSettlement for MockGame {
        fn update_settlement(&mut self, settlement: Settlement) {
            self.updated_settlements.push(settlement)
        }
    }

    fn add_route(
        route_key: RouteKey,
        route: Route,
        routes: &mut HashMap<RouteKey, Route>,
        traffic: &mut Traffic,
    ) {
        for position in route.path.iter() {
            traffic.mut_cell_unsafe(position).insert(route_key);
        }

        routes.insert(route_key, route);
    }

    #[test]
    fn should_update_settlement_population_based_on_routes_ending_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    ..Settlement::default()
                },
                v2(3, 3) => Settlement{
                    position: v2(3, 3),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };

        let route_key_1 = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route_1 = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 39,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_1, route_1, &mut game.routes, &mut traffic);

        let route_key_2 = RouteKey {
            settlement: v2(3, 3),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
            traffic: 17,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_2, route_2, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            params: SimulationParams {
                traffic_to_population: 0.5,
                ..SimulationParams::default()
            },
            traffic,
            route_to_ports: hashmap! {
                route_key_1 => hashset!{ v2(0, 0), v2(1, 0) },
            },
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 30.0 * state.params.traffic_to_population, // sum( traffic / ( total ports on route + 1 ) )
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_update_settlement_population_based_on_ports_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 0), v2(2, 1), v2(2, 2), v2(2, 3) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(3, 1),
        };
        let route = Route {
            path: vec![v2(0, 1), v2(1, 1), v2(2, 1), v2(3, 1)],
            traffic: 33,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            params: SimulationParams {
                traffic_to_population: 0.5,
                ..SimulationParams::default()
            },
            traffic,
            route_to_ports: hashmap! {
                route_key => hashset!{v2(0, 1), v2(2, 1)}
            },
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 11.0 * state.params.traffic_to_population, // sum( traffic / ( total ports on route + 1 ) )
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_add_routes_ending_in_territory_to_ports_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    ..Settlement::default()
                },
                v2(3, 3) => Settlement{
                    position: v2(3, 3),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };

        let route_key_1 = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route_1 = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 38,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_1, route_1, &mut game.routes, &mut traffic);

        let route_key_2 = RouteKey {
            settlement: v2(3, 3),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
            traffic: 14,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_2, route_2, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            params: SimulationParams {
                traffic_to_population: 0.5,
                ..SimulationParams::default()
            },
            traffic,
            route_to_ports: hashmap! {
                route_key_1 => hashset!{ v2(0, 0) },
                route_key_2 => hashset!{ v2(3, 2) },
            },
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 33.0 * state.params.traffic_to_population, // sum( traffic / ( total ports on route + 1 ) )
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_ignore_routes_not_ending_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            target_population: 10.0,
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(3, 3),
        };
        let route = Route {
            path: vec![
                v2(0, 0),
                v2(1, 0),
                v2(2, 0),
                v2(3, 0),
                v2(3, 1),
                v2(3, 2),
                v2(3, 3),
            ],
            traffic: 13,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 0.0,
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_ignore_routes_starting_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            target_population: 10.0,
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(3, 1) => Settlement{
                    position: v2(0, 0),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };
        let route_key = RouteKey {
            settlement: v2(3, 1),
            resource: Resource::Gems,
            destination: v2(3, 2),
        };
        let route = Route {
            path: vec![v2(3, 1), v2(3, 2)],
            traffic: 13,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 0.0,
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_ignore_ports_outside_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            target_population: 10.0,
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 0), v2(2, 1), v2(2, 2), v2(2, 3) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 1) => Settlement{
                    position: v2(0, 1),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };

        let route_key = RouteKey {
            settlement: v2(0, 1),
            resource: Resource::Gems,
            destination: v2(3, 1),
        };
        let route = Route {
            path: vec![v2(0, 1), v2(1, 1), v2(2, 1), v2(3, 1)],
            traffic: 32,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            traffic,
            route_to_ports: hashmap! {
                route_key => hashset! { v2(0, 1) }
            },
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 0.0,
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_set_target_to_zero_if_no_routes() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            target_population: 10.0,
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 0), v2(2, 1), v2(2, 2), v2(2, 3) };
        let game = MockGame::default();

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(State::default(), &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 0.0,
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_ignore_invalid_route() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame::default();

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 39,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);
        game.routes = hashmap! {}; // Removing route to create invalid state

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 0.0,
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_change_nation_if_majority_of_traffic_share_from_one_nation() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            nation: "A".to_string(),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "B".to_string(),
                    ..Settlement::default()
                },
                v2(3, 3) => Settlement{
                    position: v2(3, 3),
                    nation: "C".to_string(),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };

        let route_key_1 = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route_1 = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 68,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_1, route_1, &mut game.routes, &mut traffic);

        let route_key_2 = RouteKey {
            settlement: v2(3, 3),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
            traffic: 32,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_2, route_2, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            params: SimulationParams {
                traffic_to_population: 0.5,
                nation_flip_traffic_pc: 0.67,
            },
            traffic,
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        assert_eq!(game.updated_settlements[0].nation, "B".to_string());
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_not_change_nation_if_no_nation_has_majority_traffic_share() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            nation: "A".to_string(),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "B".to_string(),
                    ..Settlement::default()
                },
                v2(3, 3) => Settlement{
                    position: v2(3, 3),
                    nation: "C".to_string(),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };

        let route_key_1 = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route_1 = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 60,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_1, route_1, &mut game.routes, &mut traffic);

        let route_key_2 = RouteKey {
            settlement: v2(3, 3),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
            traffic: 40,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_2, route_2, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            params: SimulationParams {
                traffic_to_population: 0.5,
                nation_flip_traffic_pc: 0.67,
            },
            traffic,
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        assert_eq!(game.updated_settlements[0].nation, "A".to_string());
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_link_route_from_corner_of_settlement_to_correct_settlement() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
        };

        let route_key = RouteKey {
            settlement: v2(1, 1),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route = Route {
            path: vec![v2(1, 1), v2(2, 1), v2(2, 2)],
            traffic: 10,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            params: SimulationParams {
                traffic_to_population: 0.5,
                ..SimulationParams::default()
            },
            traffic,
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 10.0 * state.params.traffic_to_population, // sum( traffic / ( total ports on route + 1 ) )
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }

    #[test]
    fn should_ignore_route_from_invalid_settlement() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame::default();

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 39,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = UpdateTown::new(&game.tx());

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 0.0,
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );
    }
}
