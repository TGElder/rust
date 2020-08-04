use super::*;

use crate::game::traits::{HasWorld, Micros, Routes};
use crate::route::RouteKey;
use crate::travel_duration::TravelDuration;
use commons::edge::Edge;
use std::collections::HashSet;

pub fn get_edge_traffic<G, T>(
    game: &G,
    travel_duration: &T,
    state: &State,
    edge: &Edge,
) -> EdgeTrafficSummary
where
    G: HasWorld + Micros + Routes,
    T: TravelDuration + 'static,
{
    let route_keys = get_route_keys(&state, &edge);
    get_edge_traffic_with_route_keys(game, travel_duration, &edge, &route_keys)
}

fn get_route_keys(state: &State, edge: &Edge) -> HashSet<RouteKey> {
    state
        .edge_traffic
        .get(edge)
        .cloned()
        .unwrap_or_else(HashSet::new)
}

fn get_edge_traffic_with_route_keys<G, T>(
    game: &G,
    travel_duration: &T,
    edge: &Edge,
    route_keys: &HashSet<RouteKey>,
) -> EdgeTrafficSummary
where
    G: HasWorld + Micros + Routes,
    T: TravelDuration + 'static,
{
    EdgeTrafficSummary {
        edge: *edge,
        road_status: get_road_status(game, travel_duration, edge),
        routes: get_routes(game, route_keys),
    }
}

fn get_road_status<G, T>(game: &G, travel_duration: &T, edge: &Edge) -> RoadStatus
where
    G: HasWorld,
    T: TravelDuration + 'static,
{
    let world = game.world();
    if world.is_road(edge) {
        RoadStatus::Built
    } else if let Some(when) = world.road_planned(edge) {
        RoadStatus::Planned(when)
    } else if travel_duration
        .get_duration(world, edge.from(), edge.to())
        .is_some()
    {
        RoadStatus::Suitable
    } else {
        RoadStatus::Unsuitable
    }
}

fn get_routes<G>(game: &G, route_keys: &HashSet<RouteKey>) -> Vec<EdgeRouteSummary>
where
    G: Micros + Routes,
{
    route_keys
        .iter()
        .flat_map(|route_key| get_route(game, route_key))
        .collect()
}

fn get_route<G>(game: &G, route_key: &RouteKey) -> Option<EdgeRouteSummary>
where
    G: Micros + Routes,
{
    let route = game.get_route(&route_key)?;
    Some(EdgeRouteSummary {
        traffic: route.traffic,
        first_visit: game.micros() + route.duration.as_micros(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::route::{Route, RouteSet, RouteSetKey};
    use crate::travel_duration::ConstantTravelDuration;
    use crate::world::World;
    use commons::same_elements;
    use commons::{v2, M};
    use std::collections::HashMap;
    use std::time::Duration;

    fn world() -> World {
        World::new(M::zeros(4, 4), 0.0)
    }

    struct MockGame {
        micros: u128,
        world: World,
        routes: HashMap<RouteSetKey, RouteSet>,
    }

    impl Default for MockGame {
        fn default() -> MockGame {
            MockGame {
                micros: 0,
                world: world(),
                routes: HashMap::new(),
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

    impl Routes for MockGame {
        fn routes(&self) -> &HashMap<RouteSetKey, RouteSet> {
            &self.routes
        }

        fn routes_mut(&mut self) -> &mut HashMap<RouteSetKey, RouteSet> {
            &mut self.routes
        }
    }

    struct NullTravelDuration {}

    impl TravelDuration for NullTravelDuration {
        fn get_duration(&self, _: &World, _: &V2<usize>, _: &V2<usize>) -> Option<Duration> {
            None
        }

        fn min_duration(&self) -> Duration {
            Duration::from_secs(0)
        }

        fn max_duration(&self) -> Duration {
            Duration::from_secs(0)
        }
    }

    fn route_set(route_key: RouteKey, route: Route) -> (RouteSetKey, RouteSet) {
        let route_set_key = (&route_key).into();
        let route_set = hashmap! {
            route_key => route
        };
        (route_set_key, route_set)
    }

    #[test]
    fn edge() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let game = MockGame::default();

        // When
        let traffic = get_edge_traffic(&game, &NullTravelDuration {}, &State::default(), &edge);

        // Then
        assert_eq!(traffic.edge, edge);
    }

    #[test]
    fn road_status_built() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let mut world = world();
        world.set_road(&edge, true);
        let game = MockGame {
            world,
            ..MockGame::default()
        };

        // When
        let traffic = get_edge_traffic(&game, &NullTravelDuration {}, &State::default(), &edge);

        // Then
        assert_eq!(traffic.road_status, RoadStatus::Built);
    }

    #[test]
    fn road_status_planned() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let mut world = world();
        world.plan_road(&edge, true, 404);
        let game = MockGame {
            world,
            ..MockGame::default()
        };

        // When
        let traffic = get_edge_traffic(&game, &NullTravelDuration {}, &State::default(), &edge);

        // Then
        assert_eq!(traffic.road_status, RoadStatus::Planned(404));
    }

    #[test]
    fn road_status_unsuitable() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let game = MockGame::default();

        // When
        let traffic = get_edge_traffic(&game, &NullTravelDuration {}, &State::default(), &edge);

        // Then
        assert_eq!(traffic.road_status, RoadStatus::Unsuitable);
    }

    #[test]
    fn road_status_suitable() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let game = MockGame::default();

        // When
        let traffic = get_edge_traffic(
            &game,
            &ConstantTravelDuration::new(Duration::from_secs(0)),
            &State::default(),
            &edge,
        );

        // Then
        assert_eq!(traffic.road_status, RoadStatus::Suitable);
    }

    #[test]
    fn routes() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let route_key_1 = RouteKey {
            settlement: v2(1, 1),
            resource: Resource::Wood,
            destination: v2(1, 3),
        };
        let route_1 = Route {
            path: vec![],
            start_micros: 0,
            duration: Duration::from_micros(101),
            traffic: 11,
        };
        let (route_set_key_1, route_set_1) = route_set(route_key_1, route_1);
        let route_key_2 = RouteKey {
            settlement: v2(1, 3),
            resource: Resource::Wood,
            destination: v2(0, 2),
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
        let state = State {
            edge_traffic: hashmap! {
                edge => hashset!{route_key_1, route_key_2},
            },
            ..State::default()
        };

        let game = MockGame {
            micros: 1000,
            routes,
            ..MockGame::default()
        };

        // When
        let traffic = get_edge_traffic(&game, &NullTravelDuration {}, &state, &edge);

        // Then
        assert!(same_elements(
            &traffic.routes,
            &[
                EdgeRouteSummary {
                    traffic: 11,
                    first_visit: 1101,
                },
                EdgeRouteSummary {
                    traffic: 22,
                    first_visit: 1202,
                },
            ]
        ));
    }

    #[test]
    fn no_routes() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let game = MockGame::default();

        // When
        let traffic = get_edge_traffic(&game, &NullTravelDuration {}, &State::default(), &edge);

        // Then
        assert_eq!(traffic.routes, vec![]);
    }

    #[test]
    fn non_existent_route() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let route_key = RouteKey {
            settlement: v2(1, 1),
            resource: Resource::Wood,
            destination: v2(1, 3),
        };
        let state = State {
            edge_traffic: hashmap! {
                edge => hashset!{route_key},
            },
            ..State::default()
        };

        let game = MockGame::default();

        // When
        let traffic = get_edge_traffic(&game, &NullTravelDuration {}, &state, &edge);

        // Then
        assert_eq!(traffic.routes, vec![]);
    }
}
