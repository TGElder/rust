use super::*;

use crate::avatar::CheckForPort;
use crate::game::traits::HasWorld;
use crate::route::{Route, RouteKey};
use commons::edge::Edges;
use std::collections::HashSet;

const HANDLE: &str = "update_route_to_ports";

pub struct UpdateRouteToPorts<G>
where
    G: CheckForPort + HasWorld,
{
    game: UpdateSender<G>,
}

impl<G> Processor for UpdateRouteToPorts<G>
where
    G: CheckForPort + HasWorld,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let route_changes = match instruction {
            Instruction::ProcessRouteChanges(route_changes) => (route_changes.clone()),
            _ => return state,
        };
        self.update_many_route_to_ports(state, route_changes)
    }
}

impl<G> UpdateRouteToPorts<G>
where
    G: CheckForPort + HasWorld,
{
    pub fn new(game: &UpdateSender<G>) -> UpdateRouteToPorts<G> {
        UpdateRouteToPorts {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn update_many_route_to_ports(
        &mut self,
        state: State,
        route_changes: Vec<RouteChange>,
    ) -> State {
        block_on(async {
            self.game
                .update(move |game| update_many_route_to_ports(game, state, route_changes))
                .await
        })
    }
}

pub fn update_many_route_to_ports<G>(
    game: &mut G,
    mut state: State,
    route_changes: Vec<RouteChange>,
) -> State
where
    G: CheckForPort + HasWorld,
{
    for route_change in route_changes {
        update_route_to_ports(game, &mut state, &route_change);
    }
    state
}

pub fn update_route_to_ports<G>(game: &G, state: &mut State, route_change: &RouteChange)
where
    G: CheckForPort + HasWorld,
{
    match route_change {
        RouteChange::New { key, route } => update(game, state, key, route),
        RouteChange::Updated { key, new, old } if new.path != old.path => {
            update(game, state, key, new)
        }
        RouteChange::Removed { key, .. } => remove(state, key),
        _ => (),
    }
}

fn update<G>(game: &G, state: &mut State, route_key: &RouteKey, route: &Route)
where
    G: CheckForPort + HasWorld,
{
    let ports = get_ports(game, &route.path);
    if ports.is_empty() {
        remove(state, route_key);
    } else {
        state.route_to_ports.insert(*route_key, ports);
    }
}

fn get_ports<G>(game: &G, path: &[V2<usize>]) -> HashSet<V2<usize>>
where
    G: CheckForPort + HasWorld,
{
    let world = game.world();
    path.edges()
        .flat_map(|edge| game.check_for_port(world, edge.from(), edge.to()))
        .collect()
}

fn remove(state: &mut State, route_key: &RouteKey) {
    state.route_to_ports.remove(route_key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::Resource;
    use crate::route::Route;
    use crate::world::World;
    use commons::update::UpdateProcess;
    use commons::{v2, M};
    use std::time::Duration;

    fn world() -> World {
        World::new(M::zeros(3, 3), 0.0)
    }

    struct MockGame {
        ports: HashSet<V2<usize>>,
        world: World,
    }

    impl Default for MockGame {
        fn default() -> MockGame {
            MockGame {
                ports: hashset! {},
                world: world(),
            }
        }
    }

    impl CheckForPort for MockGame {
        fn check_for_port(&self, _: &World, from: &V2<usize>, _: &V2<usize>) -> Option<V2<usize>> {
            if self.ports.contains(from) {
                Some(*from)
            } else {
                None
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

    #[test]
    fn should_insert_entry_for_new_route_with_ports() {
        // Given
        let game = MockGame {
            ports: hashset! {v2(0, 1), v2(1, 2)},
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);

        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let state = State::default();

        let mut processor = UpdateRouteToPorts::new(&game.tx());

        // When
        let route_change = RouteChange::New { key, route };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = processor.process(state, &instruction);

        // Then
        assert_eq!(
            state.route_to_ports,
            hashmap! { key => hashset!{ v2(0, 1), v2(1, 2) } }
        );

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_do_nothing_for_new_route_with_no_ports() {
        // Given
        let game = MockGame::default();
        let game = UpdateProcess::new(game);

        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let state = State::default();

        let mut processor = UpdateRouteToPorts::new(&game.tx());

        // When
        let route_change = RouteChange::New { key, route };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = processor.process(state, &instruction);

        // Then
        assert_eq!(state.route_to_ports, hashmap! {});

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_update_entry_for_updated_route_with_updated_path_with_ports() {
        // Given
        let game = MockGame {
            ports: hashset! {v2(0, 1), v2(1, 0), v2(1, 2)},
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);

        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let old = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };
        let new = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let state = State {
            route_to_ports: hashmap! { key => hashset!{ v2(1, 0) } },
            ..State::default()
        };

        let mut processor = UpdateRouteToPorts::new(&game.tx());

        // When
        let route_change = RouteChange::Updated { key, old, new };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = processor.process(state, &instruction);

        // Then
        assert_eq!(
            state.route_to_ports,
            hashmap! { key => hashset!{ v2(0, 1), v2(1, 2) } }
        );

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_remove_entry_for_updated_route_with_no_ports() {
        // Given
        let game = MockGame {
            ports: hashset! {v2(1, 0)},
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);

        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let old = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };
        let new = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let state = State {
            route_to_ports: hashmap! { key => hashset!{ v2(1, 0) } },
            ..State::default()
        };

        let mut processor = UpdateRouteToPorts::new(&game.tx());

        // When
        let route_change = RouteChange::Updated { key, old, new };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = processor.process(state, &instruction);

        // Then
        assert_eq!(state.route_to_ports, hashmap! {});

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_do_nothing_for_updated_route_with_same_path() {
        // Given
        let game = MockGame {
            ports: hashset! {v2(0, 1), v2(1, 0), v2(1, 2)},
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);

        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let old = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };
        let new = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 10,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let state = State {
            route_to_ports: hashmap! {}, // Incorrect so we can check it is not corrected
            ..State::default()
        };

        let mut processor = UpdateRouteToPorts::new(&game.tx());

        // When
        let route_change = RouteChange::Updated { key, old, new };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = processor.process(state, &instruction);

        // Then
        assert_eq!(state.route_to_ports, hashmap! {});

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_remove_entry_for_removed_route() {
        // Given
        let game = MockGame::default();
        let game = UpdateProcess::new(game);

        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let state = State {
            route_to_ports: hashmap! { key => hashset!{ v2(0, 1), v2(1, 2) } },
            ..State::default()
        };

        let mut processor = UpdateRouteToPorts::new(&game.tx());

        // When
        let route_change = RouteChange::Removed { key, route };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = processor.process(state, &instruction);

        // Then
        assert_eq!(state.route_to_ports, hashmap! {});

        // Finally
        game.shutdown();
    }

    #[test]
    fn multiple_changes() {
        // Given
        let game = MockGame {
            ports: hashset! {v2(0, 1), v2(1, 2)},
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);

        let key_new = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let route_new = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };
        let key_removed = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(1, 1),
        };
        let route_removed = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(1, 1)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let state = State {
            route_to_ports: hashmap! { key_removed => hashset!{ v2(0, 1) } },
            ..State::default()
        };

        let mut processor = UpdateRouteToPorts::new(&game.tx());

        // When
        let route_changes = vec![
            RouteChange::New {
                key: key_new,
                route: route_new,
            },
            RouteChange::Removed {
                key: key_removed,
                route: route_removed,
            },
        ];
        let instruction = Instruction::ProcessRouteChanges(route_changes);
        let state = processor.process(state, &instruction);

        // Then
        assert_eq!(
            state.route_to_ports,
            hashmap! { key_new => hashset!{ v2(0, 1), v2(1, 2) } }
        );

        // Finally
        game.shutdown();
    }
}
