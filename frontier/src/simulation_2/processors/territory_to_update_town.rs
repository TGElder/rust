use super::*;
use crate::game::traits::{Routes, UpdateSettlement};
use crate::route::RouteKey;
use crate::settlement::Settlement;
use std::collections::HashSet;

const HANDLE: &str = "territory_to_settlement";
const TRAFFIC_TO_POPULATION: f64 = 0.5;

pub struct TerritoryToUpdateTown<G>
where
    G: Routes + UpdateSettlement,
{
    game: UpdateSender<G>,
}

impl<G> Processor for TerritoryToUpdateTown<G>
where
    G: Routes + UpdateSettlement,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let (settlement, territory) = match instruction {
            Instruction::Territory {
                settlement,
                territory,
            } => (settlement, territory),
            _ => return state,
        };

        let route_keys = get_route_keys(territory, &state);
        if route_keys.is_empty() {
            return state;
        }

        self.update_settlement(settlement.clone(), route_keys);

        state
    }
}

impl<G> TerritoryToUpdateTown<G>
where
    G: Routes + UpdateSettlement,
{
    pub fn new(game: &UpdateSender<G>) -> TerritoryToUpdateTown<G> {
        TerritoryToUpdateTown {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn update_settlement(&mut self, settlement: Settlement, route_keys: HashSet<RouteKey>) {
        block_on(async {
            self.game
                .update(move |game| update_settlement(game, settlement, route_keys))
                .await
        })
    }
}

fn get_route_keys(territory: &HashSet<V2<usize>>, state: &State) -> HashSet<RouteKey> {
    territory
        .iter()
        .flat_map(|position| state.traffic.get(position))
        .flatten()
        .filter(|RouteKey { settlement, .. }| !territory.contains(settlement))
        .filter(|RouteKey { destination, .. }| territory.contains(destination))
        .cloned()
        .collect()
}

fn update_settlement<G>(game: &mut G, settlement: Settlement, route_keys: HashSet<RouteKey>)
where
    G: Routes + UpdateSettlement,
{
    let traffic = get_traffic(game, route_keys);
    let settlement = Settlement {
        target_population: traffic as f64 * TRAFFIC_TO_POPULATION,
        ..settlement
    };
    game.update_settlement(settlement);
}

fn get_traffic<G>(game: &mut G, route_keys: HashSet<RouteKey>) -> usize
where
    G: Routes,
{
    route_keys
        .into_iter()
        .flat_map(|route_key| game.get_route(&route_key))
        .map(|route| route.traffic)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::route::{Route, RouteSet, RouteSetKey};
    use crate::world::Resource;
    use commons::grid::Grid;
    use commons::update::UpdateProcess;
    use commons::v2;

    use std::collections::{HashMap, HashSet};
    use std::time::Duration;

    struct MockGame {
        routes: HashMap<RouteSetKey, RouteSet>,
        updated_settlements: Vec<Settlement>,
    }

    impl MockGame {
        fn new() -> MockGame {
            MockGame {
                routes: hashmap! {},
                updated_settlements: vec![],
            }
        }
    }

    impl Routes for MockGame {
        fn routes(&self) -> &HashMap<RouteSetKey, RouteSet> {
            &self.routes
        }

        fn routes_mut(&mut self) -> &mut HashMap<RouteSetKey, RouteSet> {
            &mut self.routes
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
        routes: &mut HashMap<RouteSetKey, RouteSet>,
        traffic: &mut Traffic,
    ) {
        for position in route.path.iter() {
            traffic.mut_cell_unsafe(position).insert(route_key);
        }

        let route_set = routes
            .entry((&route_key).into())
            .or_insert_with(HashMap::new);
        route_set.insert(route_key, route);
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
        let mut game = MockGame::new();
        add_route(
            RouteKey {
                settlement: v2(0, 0),
                resource: Resource::Gems,
                destination: v2(2, 1),
            },
            Route {
                path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
                traffic: 13,
                start_micros: 0,
                duration: Duration::default(),
            },
            game.routes_mut(),
            &mut traffic,
        );
        add_route(
            RouteKey {
                settlement: v2(3, 3),
                resource: Resource::Gems,
                destination: v2(2, 2),
            },
            Route {
                path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
                traffic: 26,
                start_micros: 0,
                duration: Duration::default(),
            },
            game.routes_mut(),
            &mut traffic,
        );

        let game = UpdateProcess::new(game);
        let mut processor = TerritoryToUpdateTown::new(&game.tx());

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::Territory {
            settlement: settlement.clone(),
            territory,
        };

        // When
        processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        let expected = Settlement {
            target_population: 39.0 * TRAFFIC_TO_POPULATION,
            ..settlement
        };
        assert_eq!(game.updated_settlements, vec![expected]);
    }

    #[test]
    fn should_ignore_routes_not_ending_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame::new();
        add_route(
            RouteKey {
                settlement: v2(0, 0),
                resource: Resource::Gems,
                destination: v2(3, 3),
            },
            Route {
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
            },
            game.routes_mut(),
            &mut traffic,
        );

        let game = UpdateProcess::new(game);
        let mut processor = TerritoryToUpdateTown::new(&game.tx());

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::Territory {
            settlement,
            territory,
        };

        // When
        processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        assert_eq!(game.updated_settlements, vec![]);
    }

    #[test]
    fn should_ignore_routes_starting_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut game = MockGame::new();
        add_route(
            RouteKey {
                settlement: v2(2, 1),
                resource: Resource::Gems,
                destination: v2(3, 2),
            },
            Route {
                path: vec![v2(2, 1), v2(3, 1), v2(3, 2)],
                traffic: 13,
                start_micros: 0,
                duration: Duration::default(),
            },
            game.routes_mut(),
            &mut traffic,
        );

        let game = UpdateProcess::new(game);
        let mut processor = TerritoryToUpdateTown::new(&game.tx());

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::Territory {
            settlement,
            territory,
        };

        // When
        processor.process(state, &instruction);

        // Then
        let game = game.shutdown();
        assert_eq!(game.updated_settlements, vec![]);
    }
}
