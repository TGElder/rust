use super::*;

use crate::avatar::CheckForPort;
use crate::route::{Route, RouteKey};
use crate::traits::SendWorld;
use crate::world::World;
use commons::edge::Edges;
use std::collections::HashSet;

pub struct UpdateRouteToPorts<T, C> {
    tx: T,
    port_checker: Arc<C>,
}

#[async_trait]
impl<T, C> Processor for UpdateRouteToPorts<T, C>
where
    T: SendWorld + Send + Sync,
    C: CheckForPort + Clone + Send + Sync + 'static,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let route_changes = match instruction {
            Instruction::ProcessRouteChanges(route_changes) => (route_changes.clone()),
            _ => return state,
        };
        self.update_many_route_to_ports(state, route_changes).await
    }
}

impl<T, C> UpdateRouteToPorts<T, C>
where
    T: SendWorld,
    C: CheckForPort + Clone + Send + Sync + 'static,
{
    pub fn new(tx: T, port_checker: Arc<C>) -> UpdateRouteToPorts<T, C> {
        UpdateRouteToPorts { tx, port_checker }
    }

    async fn update_many_route_to_ports(
        &mut self,
        state: State,
        route_changes: Vec<RouteChange>,
    ) -> State {
        let port_checker = self.port_checker.clone();
        self.tx
            .send_world(move |world| {
                update_many_route_to_ports(world, port_checker, state, route_changes)
            })
            .await
    }
}

pub fn update_many_route_to_ports<C>(
    world: &World,
    port_checker: Arc<C>,
    mut state: State,
    route_changes: Vec<RouteChange>,
) -> State
where
    C: CheckForPort,
{
    for route_change in route_changes {
        update_route_to_ports(world, port_checker.as_ref(), &mut state, &route_change);
    }
    state
}

pub fn update_route_to_ports(
    world: &World,
    port_checker: &dyn CheckForPort,
    state: &mut State,
    route_change: &RouteChange,
) {
    match route_change {
        RouteChange::New { key, route } => update(world, port_checker, state, key, route),
        RouteChange::Updated { key, new, old } if new.path != old.path => {
            update(world, port_checker, state, key, new)
        }
        RouteChange::Removed { key, .. } => remove(state, key),
        _ => (),
    }
}

fn update(
    world: &World,
    port_checker: &dyn CheckForPort,
    state: &mut State,
    route_key: &RouteKey,
    route: &Route,
) {
    let ports = get_ports(world, port_checker, &route.path);
    if ports.is_empty() {
        remove(state, route_key);
    } else {
        state.route_to_ports.insert(*route_key, ports);
    }
}

fn get_ports(
    world: &World,
    port_checker: &dyn CheckForPort,
    path: &[V2<usize>],
) -> HashSet<V2<usize>> {
    path.edges()
        .flat_map(|edge| port_checker.check_for_port(world, edge.from(), edge.to()))
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
    use commons::{v2, M};
    use futures::executor::block_on;
    use std::sync::Mutex;
    use std::time::Duration;

    fn world() -> Mutex<World> {
        Mutex::new(World::new(M::zeros(3, 3), 0.0))
    }

    impl CheckForPort for HashSet<V2<usize>> {
        fn check_for_port(&self, _: &World, from: &V2<usize>, _: &V2<usize>) -> Option<V2<usize>> {
            if self.contains(from) {
                Some(*from)
            } else {
                None
            }
        }
    }

    #[test]
    fn should_insert_entry_for_new_route_with_ports() {
        // Given
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

        let mut processor =
            UpdateRouteToPorts::new(world(), Arc::new(hashset! {v2(0, 1), v2(1, 2)}));

        // When
        let route_change = RouteChange::New { key, route };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.route_to_ports,
            hashmap! { key => hashset!{ v2(0, 1), v2(1, 2) } }
        );
    }

    #[test]
    fn should_do_nothing_for_new_route_with_no_ports() {
        // Given
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

        let mut processor = UpdateRouteToPorts::new(world(), Arc::new(hashset! {}));

        // When
        let route_change = RouteChange::New { key, route };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(state.route_to_ports, hashmap! {});
    }

    #[test]
    fn should_update_entry_for_updated_route_with_updated_path_with_ports() {
        // Given
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

        let mut processor =
            UpdateRouteToPorts::new(world(), Arc::new(hashset! {v2(0, 1), v2(1, 0), v2(1, 2)}));

        // When
        let route_change = RouteChange::Updated { key, old, new };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.route_to_ports,
            hashmap! { key => hashset!{ v2(0, 1), v2(1, 2) } }
        );
    }

    #[test]
    fn should_remove_entry_for_updated_route_with_no_ports() {
        // Given
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

        let mut processor = UpdateRouteToPorts::new(world(), Arc::new(hashset! {v2(1, 0)}));

        // When
        let route_change = RouteChange::Updated { key, old, new };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(state.route_to_ports, hashmap! {});
    }

    #[test]
    fn should_do_nothing_for_updated_route_with_same_path() {
        // Given
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

        let mut processor =
            UpdateRouteToPorts::new(world(), Arc::new(hashset! {v2(0, 1), v2(1, 0), v2(1, 2)}));

        // When
        let route_change = RouteChange::Updated { key, old, new };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(state.route_to_ports, hashmap! {});
    }

    #[test]
    fn should_remove_entry_for_removed_route() {
        // Given
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

        let mut processor = UpdateRouteToPorts::new(world(), Arc::new(hashset! {}));

        // When
        let route_change = RouteChange::Removed { key, route };
        let instruction = Instruction::ProcessRouteChanges(vec![route_change]);
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(state.route_to_ports, hashmap! {});
    }

    #[test]
    fn multiple_changes() {
        // Given
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

        let mut processor =
            UpdateRouteToPorts::new(world(), Arc::new(hashset! {v2(0, 1), v2(1, 2)}));

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
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.route_to_ports,
            hashmap! { key_new => hashset!{ v2(0, 1), v2(1, 2) } }
        );
    }
}
