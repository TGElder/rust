use super::*;

use crate::game::traits::{GetRoute, HasWorld, Micros, Settlements, WhoControlsTile};
use crate::route::RouteKey;
use commons::Grid;
use std::collections::HashSet;

pub fn get_position_traffic<G>(game: &G, state: &State, position: &V2<usize>) -> TrafficSummary
where
    G: HasWorld + Micros + GetRoute + Settlements + WhoControlsTile,
{
    let route_keys = get_route_keys(&state, position);
    get_traffic_with_route_keys(game, state, &position, &route_keys)
}

fn get_route_keys(state: &State, position: &V2<usize>) -> HashSet<RouteKey> {
    state.traffic.get(position).unwrap().clone()
}

fn get_traffic_with_route_keys<G>(
    game: &G,
    state: &State,
    position: &V2<usize>,
    route_keys: &HashSet<RouteKey>,
) -> TrafficSummary
where
    G: HasWorld + Micros + GetRoute + Settlements + WhoControlsTile,
{
    TrafficSummary {
        position: *position,
        controller: game.who_controls_tile(&position).cloned(),
        routes: get_routes(game, state, route_keys),
        adjacent: get_adjacent(game, position),
    }
}

fn get_routes<G>(game: &G, state: &State, route_keys: &HashSet<RouteKey>) -> Vec<RouteSummary>
where
    G: HasWorld + Micros + GetRoute + Settlements,
{
    route_keys
        .iter()
        .flat_map(|route_key| get_route(game, state, route_key))
        .collect()
}

fn get_route<G>(game: &G, state: &State, route_key: &RouteKey) -> Option<RouteSummary>
where
    G: HasWorld + Micros + GetRoute + Settlements,
{
    let route = game.get_route(&route_key)?;
    let traffic = route.traffic;
    let origin = route_key.settlement;
    let origin_settlement = game.get_settlement(&origin)?;
    let destination = route_key.destination;
    let nation = origin_settlement.nation.clone();
    let micros = game.micros();
    let first_visit = micros + route.duration.as_micros();
    let duration = route.duration;
    let resource = route_key.resource;
    let ports = state
        .route_to_ports
        .get(route_key)
        .cloned()
        .unwrap_or_default();

    let route_summary = RouteSummary {
        traffic,
        origin,
        destination,
        nation,
        first_visit,
        duration,
        resource,
        ports,
    };

    Some(route_summary)
}

fn get_adjacent<G>(game: &G, position: &V2<usize>) -> Vec<Tile>
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
    let world = game.world();
    let corners = world.get_corners_in_bounds(&tile);
    let sea = corners.iter().any(|corner| world.is_sea(corner));
    let visible = corners
        .iter()
        .all(|corner| world.get_cell_unsafe(corner).visible);

    Tile {
        position: tile,
        sea,
        visible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::route::Route;
    use crate::settlement::Settlement;
    use crate::world::World;
    use commons::same_elements;
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

    fn state() -> State {
        State {
            traffic: Traffic::same_size_as(&world(), HashSet::with_capacity(0)),
            ..State::default()
        }
    }

    struct MockGame {
        micros: u128,
        world: World,
        routes: HashMap<RouteKey, Route>,
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

        fn world_mut(&mut self) -> &mut World {
            &mut self.world
        }
    }

    impl Micros for MockGame {
        fn micros(&self) -> &u128 {
            &self.micros
        }
    }

    impl GetRoute for MockGame {
        fn get_route(&self, route_key: &RouteKey) -> Option<&Route> {
            self.routes.get(route_key)
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

        // When
        let traffic = get_position_traffic(&game, &state(), &position);

        // Then
        assert_eq!(traffic.position, position);
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

        // When
        let traffic = get_position_traffic(&game, &state(), &position);

        // Then
        assert_eq!(traffic.controller, controller);
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
        let routes = hashmap! {route_key => route};

        let mut state = state();
        state.traffic.mut_cell_unsafe(&v2(1, 2)).insert(route_key);

        let game = MockGame {
            micros: 1000,
            routes,
            settlements: route_settlements(),
            ..MockGame::default()
        };

        // When
        let traffic = get_position_traffic(&game, &state, &position);

        // Then
        assert!(same_elements(
            &traffic.routes,
            &[RouteSummary {
                traffic: 11,
                origin: v2(0, 1),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 1101,
                duration: Duration::from_micros(101),
                resource: Resource::Wood,
                ports: hashset! {},
            }],
        ));
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
        let route_key_2 = RouteKey {
            settlement: v2(0, 2),
            resource: Resource::Spice,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![],
            start_micros: 0,
            duration: Duration::from_micros(202),
            traffic: 22,
        };
        let routes = hashmap! {
            route_key_1 => route_1,
            route_key_2 => route_2
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

        // When
        let traffic = get_position_traffic(&game, &state, &position);

        // Then
        assert!(same_elements(
            &traffic.routes,
            &[
                RouteSummary {
                    traffic: 11,
                    origin: v2(0, 1),
                    destination: v2(1, 2),
                    nation: "Scotland".to_string(),
                    first_visit: 1101,
                    duration: Duration::from_micros(101),
                    resource: Resource::Wood,
                    ports: hashset! {},
                },
                RouteSummary {
                    traffic: 22,
                    origin: v2(0, 2),
                    destination: v2(2, 2),
                    nation: "Wales".to_string(),
                    first_visit: 1202,
                    duration: Duration::from_micros(202),
                    resource: Resource::Spice,
                    ports: hashset! {},
                },
            ],
        ));
    }

    #[test]
    fn ports() {
        // Given
        let position = v2(1, 2);

        let route_key = RouteKey {
            settlement: v2(0, 1),
            resource: Resource::Wood,
            destination: v2(1, 2),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(0, 3), v2(0, 4)],
            start_micros: 0,
            duration: Duration::from_micros(101),
            traffic: 11,
        };
        let routes = hashmap! {route_key => route};

        let mut state = state();
        state.traffic.mut_cell_unsafe(&v2(1, 2)).insert(route_key);
        state
            .route_to_ports
            .insert(route_key, hashset! {v2(0, 1), v2(0, 2)});

        let game = MockGame {
            micros: 1000,
            routes,
            settlements: route_settlements(),
            ..MockGame::default()
        };

        // When
        let traffic = get_position_traffic(&game, &state, &position);

        // Then
        assert_eq!(traffic.routes[0].ports, hashset! {v2(0, 1), v2(0, 2)});
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

        // When
        let traffic = get_position_traffic(&game, &state, &position);

        // Then
        assert_eq!(traffic.routes, vec![]);
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
        let routes = hashmap! {route_key => route};

        let game = MockGame {
            micros: 1000,
            routes,
            ..MockGame::default()
        };

        // When
        let traffic = get_position_traffic(&game, &state(), &position);

        // Then
        assert_eq!(traffic.routes, vec![]);
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

        // When
        let traffic = get_position_traffic(&game, &state(), &position);

        // Then
        let expected: Vec<Tile> = vec![v2(1, 2), v2(0, 2), v2(1, 1), v2(0, 1)]
            .into_iter()
            .map(|position| Tile {
                position,
                sea: position == v2(0, 1),
                visible: false,
            })
            .collect();
            assert!(same_elements(&traffic.adjacent, &expected));

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

        let state = state();

        // When
        let traffic = get_position_traffic(&game, &state, &position);

        // Then
        let expected: Vec<Tile> = vec![v2(1, 2), v2(0, 2), v2(1, 1), v2(0, 1)]
            .into_iter()
            .map(|position| Tile {
                position,
                sea: false,
                visible: position == v2(0, 1) || position == v2(0, 2),
            })
            .collect();
        assert!(same_elements(&traffic.adjacent, &expected));
    }
}
