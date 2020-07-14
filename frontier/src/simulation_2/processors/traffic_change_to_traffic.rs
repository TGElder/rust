use super::*;

use crate::commons::Grid;
use crate::game::traits::{HasWorld, Micros, Routes, Settlements, WhoControlsTile};
use crate::route::RouteKey;
use std::collections::HashSet;

const HANDLE: &str = "traffic_change_to_traffic";

pub struct TrafficChangeToTraffic<G>
where
    G: HasWorld + Micros + Routes + Settlements + WhoControlsTile,
{
    game: UpdateSender<G>,
}

impl<G> Processor for TrafficChangeToTraffic<G>
where
    G: HasWorld + Micros + Routes + Settlements + WhoControlsTile,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let position = match instruction {
            Instruction::TrafficChange(position) => *position,
            _ => return state,
        };
        let route_keys = get_route_keys(&state, &position);
        state
            .instructions
            .push(self.get_traffic(position, route_keys));
        state
    }
}

impl<G> TrafficChangeToTraffic<G>
where
    G: HasWorld + Micros + Routes + Settlements + WhoControlsTile,
{
    pub fn new(game: &UpdateSender<G>) -> TrafficChangeToTraffic<G> {
        TrafficChangeToTraffic {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn get_traffic(&mut self, position: V2<usize>, route_keys: HashSet<RouteKey>) -> Instruction {
        block_on(async {
            self.game
                .update(move |game| get_traffic(game, position, route_keys))
                .await
        })
    }
}

fn get_route_keys(state: &State, position: &V2<usize>) -> HashSet<RouteKey> {
    state.traffic.get(position).unwrap().clone()
}

fn get_traffic<G>(game: &G, position: V2<usize>, route_keys: HashSet<RouteKey>) -> Instruction
where
    G: HasWorld + Micros + Routes + Settlements + WhoControlsTile,
{
    Instruction::Traffic {
        position,
        controller: game.who_controls_tile(&position).cloned(),
        routes: get_routes(game, route_keys),
        adjacent: get_adjacent(game, position),
    }
}

fn get_routes<G>(game: &G, route_keys: HashSet<RouteKey>) -> Vec<RouteSummary>
where
    G: Micros + Routes + Settlements,
{
    route_keys
        .into_iter()
        .flat_map(|route_key| get_route(game, route_key))
        .collect()
}

fn get_route<G>(game: &G, route_key: RouteKey) -> Option<RouteSummary>
where
    G: Micros + Routes + Settlements,
{
    let route = unwrap_or!(game.get_route(&route_key), return None);
    let traffic = route.traffic;
    let origin = route_key.settlement;
    let origin_settlement = unwrap_or!(game.get_settlement(&origin), return None);
    let destination = route_key.destination;
    let nation = origin_settlement.nation.clone();
    let micros = game.micros();
    let first_visit = micros + route.duration.as_micros();
    let duration = route.duration;

    let route_summary = RouteSummary {
        traffic,
        origin,
        destination,
        nation,
        first_visit,
        duration,
    };

    Some(route_summary)
}

fn get_adjacent<G>(game: &G, position: V2<usize>) -> Vec<Tile>
where
    G: HasWorld + Settlements,
{
    let world = game.world();
    world
        .get_adjacent_tiles_in_bounds(&position)
        .into_iter()
        .map(|tile| get_tile(game, tile))
        .collect()
}

fn get_tile<G>(game: &G, tile: V2<usize>) -> Tile
where
    G: HasWorld + Settlements,
{
    let settlement = game.get_settlement(&tile).cloned();
    let world = game.world();
    let corners = world.get_corners_in_bounds(&tile);
    let sea = corners.iter().any(|corner| world.is_sea(corner));
    let visible = corners
        .iter()
        .all(|corner| world.get_cell_unsafe(corner).visible);

    Tile {
        position: tile,
        settlement,
        sea,
        visible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::route::{Route, RouteSet, RouteSetKey};
    use crate::settlement::Settlement;
    use crate::world::{Resource, World};
    use commons::same_elements;
    use commons::update::UpdateProcess;
    use commons::{v2, M};
    use std::collections::HashMap;
    use std::time::Duration;

    fn world() -> World {
        World::new(M::zeros(3, 3), -0.5)
    }

    fn route_settlements() -> HashMap<V2<usize>, Settlement> {
        hashmap! {
            v2(0, 0) => Settlement{
                position: v2(0, 0),
                nation: "Scotland".to_string(),
                ..Settlement::default()
            },
            v2(0, 1) => Settlement{
                position: v2(0, 1),
                nation: "Scotland".to_string(),
                ..Settlement::default()
            },
            v2(0, 2) => Settlement{
                position: v2(0, 1),
                nation: "Wales".to_string(),
                ..Settlement::default()
            }
        }
    }

    fn route_set(route_key: RouteKey, route: Route) -> (RouteSetKey, RouteSet) {
        let route_set_key = (&route_key).into();
        let route_set = hashmap! {
            route_key => route
        };
        (route_set_key, route_set)
    }

    fn state() -> State {
        State {
            traffic: Traffic::same_size_as(&world(), HashSet::with_capacity(0)),
            ..State::default()
        }
    }

    struct MockGame {
        micros: u128,
        world: World,
        routes: HashMap<RouteSetKey, RouteSet>,
        settlements: HashMap<V2<usize>, Settlement>,
        controller: Option<V2<usize>>,
    }

    impl Default for MockGame {
        fn default() -> MockGame {
            MockGame {
                micros: 0,
                world: world(),
                routes: HashMap::new(),
                settlements: HashMap::new(),
                controller: None,
            }
        }
    }

    impl HasWorld for MockGame {
        fn world(&self) -> &World {
            &self.world
        }
    }

    impl Micros for MockGame {
        fn micros(&self) -> &u128 {
            &self.micros
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

    impl Settlements for MockGame {
        fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
            self.settlements.settlements()
        }

        fn get_settlement(&self, position: &V2<usize>) -> Option<&Settlement> {
            self.settlements.get_settlement(position)
        }
    }

    impl WhoControlsTile for MockGame {
        fn who_controls_tile(&self, _: &V2<usize>) -> Option<&V2<usize>> {
            self.controller.as_ref()
        }
    }

    #[test]
    fn position() {
        // Given
        let position = v2(1, 2);
        let game = MockGame::default();
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        // When
        let state = processor.process(state(), &Instruction::TrafficChange(position));

        // Then
        if let Some(Instruction::Traffic {
            position: actual, ..
        }) = state.instructions.get(0)
        {
            assert_eq!(*actual, position);
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[test]
    fn controller() {
        // Given
        let position = v2(1, 2);
        let controller = Some(v2(1, 0));
        let game = MockGame {
            controller,
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        // When
        let state = processor.process(state(), &Instruction::TrafficChange(position));

        // Then
        if let Some(Instruction::Traffic {
            controller: actual, ..
        }) = state.instructions.get(0)
        {
            assert_eq!(*actual, controller);
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[test]
    fn traffic_one_route() {
        // Given
        let position = v2(1, 2);

        let route_key = RouteKey {
            settlement: v2(0, 1),
            resource: Resource::Wood,
            destination: v2(1, 2),
        };
        let route = Route {
            path: vec![],
            start_micros: 0,
            duration: Duration::from_micros(101),
            traffic: 11,
        };
        let (route_set_key, route_set) = route_set(route_key, route);
        let routes = hashmap! {route_set_key => route_set};

        let mut state = state();
        state.traffic.mut_cell_unsafe(&v2(1, 2)).insert(route_key);

        let game = MockGame {
            micros: 1000,
            routes,
            settlements: route_settlements(),
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        // When
        let state = processor.process(state, &Instruction::TrafficChange(position));

        // Then
        if let Some(Instruction::Traffic { routes, .. }) = state.instructions.get(0) {
            assert!(same_elements(
                routes,
                &[RouteSummary {
                    traffic: 11,
                    origin: v2(0, 1),
                    destination: v2(1, 2),
                    nation: "Scotland".to_string(),
                    first_visit: 1101,
                    duration: Duration::from_micros(101),
                }],
            ));
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[test]
    fn traffic_two_routes() {
        // Given
        let position = v2(1, 2);

        let route_key_1 = RouteKey {
            settlement: v2(0, 1),
            resource: Resource::Wood,
            destination: v2(1, 2),
        };
        let route_1 = Route {
            path: vec![],
            start_micros: 0,
            duration: Duration::from_micros(101),
            traffic: 11,
        };
        let (route_set_key_1, route_set_1) = route_set(route_key_1, route_1);
        let route_key_2 = RouteKey {
            settlement: v2(0, 2),
            resource: Resource::Wood,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![],
            start_micros: 0,
            duration: Duration::from_micros(202),
            traffic: 22,
        };
        let (route_set_key_2, route_set_2) = route_set(route_key_2, route_2);
        let routes = hashmap! {
            route_set_key_1 => route_set_1,
            route_set_key_2 => route_set_2
        };

        let mut state = state();
        state.traffic.mut_cell_unsafe(&v2(1, 2)).insert(route_key_1);
        state.traffic.mut_cell_unsafe(&v2(1, 2)).insert(route_key_2);

        let game = MockGame {
            micros: 1000,
            routes,
            settlements: route_settlements(),
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        // When
        let state = processor.process(state, &Instruction::TrafficChange(position));

        // Then
        if let Some(Instruction::Traffic { routes, .. }) = state.instructions.get(0) {
            assert!(same_elements(
                routes,
                &[
                    RouteSummary {
                        traffic: 11,
                        origin: v2(0, 1),
                        destination: v2(1, 2),
                        nation: "Scotland".to_string(),
                        first_visit: 1101,
                        duration: Duration::from_micros(101),
                    },
                    RouteSummary {
                        traffic: 22,
                        origin: v2(0, 2),
                        destination: v2(2, 2),
                        nation: "Wales".to_string(),
                        first_visit: 1202,
                        duration: Duration::from_micros(202),
                    },
                ],
            ));
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[test]
    fn traffic_non_existent_route() {
        // Given
        let position = v2(1, 2);

        let route_key = RouteKey {
            settlement: v2(0, 1),
            resource: Resource::Wood,
            destination: v2(1, 2),
        };

        let mut state = state();
        state.traffic.mut_cell_unsafe(&position).insert(route_key);

        let game = MockGame {
            micros: 1000,
            settlements: route_settlements(),
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        // When
        let state = processor.process(state, &Instruction::TrafficChange(position));

        // Then
        if let Some(Instruction::Traffic { routes, .. }) = state.instructions.get(0) {
            assert_eq!(*routes, vec![]);
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[test]
    fn traffic_non_existent_settlement() {
        // Given
        let position = v2(1, 2);

        let route_key = RouteKey {
            settlement: v2(1, 1),
            resource: Resource::Wood,
            destination: v2(1, 2),
        };
        let route = Route {
            path: vec![],
            start_micros: 0,
            duration: Duration::from_micros(101),
            traffic: 11,
        };
        let (route_set_key, route_set) = route_set(route_key, route);
        let routes = hashmap! {route_set_key => route_set};

        let game = MockGame {
            micros: 1000,
            routes,
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        // When
        let state = processor.process(state(), &Instruction::TrafficChange(position));

        // Then
        if let Some(Instruction::Traffic { routes, .. }) = state.instructions.get(0) {
            assert_eq!(*routes, vec![]);
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[test]
    fn settlement() {
        // Given
        let position = v2(1, 2);
        let settlements: HashMap<V2<usize>, Settlement> =
            vec![v2(1, 2), v2(0, 2), v2(1, 1), v2(0, 1)]
                .into_iter()
                .map(|position| {
                    (
                        position,
                        Settlement {
                            position,
                            ..Settlement::default()
                        },
                    )
                })
                .collect();

        let game = MockGame {
            settlements: settlements.clone(),
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        // When
        let state = processor.process(state(), &Instruction::TrafficChange(position));

        // Then
        let expected: Vec<Tile> = settlements
            .into_iter()
            .map(|(_, settlement)| Tile {
                position: settlement.position,
                settlement: Some(settlement),
                sea: false,
                visible: false,
            })
            .collect();
        if let Some(Instruction::Traffic {
            adjacent: actual, ..
        }) = state.instructions.get(0)
        {
            assert!(same_elements(&actual, &expected));
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[rustfmt::skip]
    #[test]
    fn sea() {
        // Given
        let position = v2(1, 2);
        let world = World::new(
            M::from_vec(3, 3, vec![
                0.0, 1.0, 1.0,
                0.0, 1.0, 1.0,
                1.0, 1.0, 1.0]),
            0.5,
        );

        let game = MockGame {
            world,
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        // When
        let state = processor.process(state(), &Instruction::TrafficChange(position));

        // Then
        let expected: Vec<Tile> = vec![v2(1, 2), v2(0, 2), v2(1, 1), v2(0, 1)]
            .into_iter()
            .map(|position| Tile {
                position,
                settlement: None,
                sea: position == v2(0, 1),
                visible: false,
            })
            .collect();
        if let Some(Instruction::Traffic {
            adjacent: actual, ..
        }) = state.instructions.get(0)
        {
            assert!(same_elements(&actual, &expected));
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[test]
    fn visible() {
        // Given
        let position = v2(1, 2);
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).visible = true;
        world.mut_cell_unsafe(&v2(0, 2)).visible = true;
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        world.mut_cell_unsafe(&v2(0, 1)).visible = true;

        let game = MockGame {
            world,
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut processor = TrafficChangeToTraffic::new(&game.tx());

        let state = state();

        // When
        let state = processor.process(state, &Instruction::TrafficChange(position));

        // Then
        let expected: Vec<Tile> = vec![v2(1, 2), v2(0, 2), v2(1, 1), v2(0, 1)]
            .into_iter()
            .map(|position| Tile {
                position,
                settlement: None,
                sea: false,
                visible: position == v2(0, 1) || position == v2(0, 2),
            })
            .collect();
        if let Some(Instruction::Traffic {
            adjacent: actual, ..
        }) = state.instructions.get(0)
        {
            assert!(same_elements(&actual, &expected));
        } else {
            panic!("No traffic instruction!");
        }

        // Finally
        game.shutdown();
    }
}
