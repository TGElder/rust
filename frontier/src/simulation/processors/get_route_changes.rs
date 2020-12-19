use super::*;

use crate::game::traits::Routes;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

const NAME: &str = "get_route_changes";

pub struct GetRouteChanges<G>
where
    G: Routes + Send,
{
    game: FnSender<G>,
}

#[async_trait]
impl<G> Processor for GetRouteChanges<G>
where
    G: Routes + Send,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let (key, route_set) = match instruction {
            Instruction::GetRouteChanges { key, route_set } => (*key, route_set.clone()),
            _ => return state,
        };
        let route_changes = self.update_routes_and_get_changes(key, route_set).await;
        if route_changes.is_empty() {
            return state;
        }
        state
            .instructions
            .push(Instruction::ProcessRouteChanges(route_changes));
        state
    }
}

impl<G> GetRouteChanges<G>
where
    G: Routes + Send,
{
    pub fn new(game: &FnSender<G>) -> GetRouteChanges<G> {
        GetRouteChanges {
            game: game.clone_with_name(NAME),
        }
    }

    async fn update_routes_and_get_changes(
        &mut self,
        key: RouteSetKey,
        route_set: RouteSet,
    ) -> Vec<RouteChange> {
        self.game
            .send(move |game| update_routes_and_get_changes(game, key, route_set))
            .await
    }
}

pub fn update_routes_and_get_changes(
    routes: &mut dyn Routes,
    key: RouteSetKey,
    route_set: RouteSet,
) -> Vec<RouteChange> {
    let mut new_and_changed = add_and_get_new_and_changed(routes, &key, &route_set);
    let mut removed = remove_and_get_removed(routes, &key, &route_set);
    let mut out = Vec::with_capacity(new_and_changed.len() + removed.len());
    out.append(&mut new_and_changed);
    out.append(&mut removed);
    out
}

fn add_and_get_new_and_changed(
    routes: &mut dyn Routes,
    set_key: &RouteSetKey,
    route_set: &RouteSet,
) -> Vec<RouteChange> {
    route_set
        .iter()
        .flat_map(move |(key, route)| add_and_get_change(routes, *set_key, *key, route.clone()))
        .collect()
}

fn add_and_get_change(
    routes: &mut dyn Routes,
    set_key: RouteSetKey,
    key: RouteKey,
    route: Route,
) -> Option<RouteChange> {
    let route_set = routes
        .routes_mut()
        .entry(set_key)
        .or_insert_with(HashMap::new);
    match route_set.entry(key) {
        Entry::Occupied(mut entry) => {
            if *entry.get() == route {
                Some(RouteChange::NoChange { key, route })
            } else {
                let old = entry.insert(route.clone());
                Some(RouteChange::Updated {
                    key,
                    old,
                    new: route,
                })
            }
        }
        Entry::Vacant(entry) => {
            entry.insert(route.clone());
            Some(RouteChange::New { key, route })
        }
    }
}

fn remove_and_get_removed(
    routes: &mut dyn Routes,
    set_key: &RouteSetKey,
    new_route_set: &RouteSet,
) -> Vec<RouteChange> {
    let old_route_set = routes
        .routes_mut()
        .entry(*set_key)
        .or_insert_with(HashMap::new);
    let new_keys: HashSet<RouteKey> = new_route_set.keys().cloned().collect();
    let old_keys: HashSet<RouteKey> = old_route_set.keys().cloned().collect();
    let to_remove = old_keys.difference(&new_keys);
    let mut out = vec![];
    for key in to_remove {
        let route = old_route_set.remove(key).unwrap();
        out.push(RouteChange::Removed { key: *key, route });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use commons::fn_sender::FnThread;
    use commons::same_elements;
    use commons::v2;
    use futures::executor::block_on;
    use std::time::Duration;

    #[test]
    fn should_add_route_and_new_route_change_if_route_is_new() {
        // Given
        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };

        let routes = hashmap! {};

        let route_set = hashmap! {
            key => route.clone()
        };

        let game = FnThread::new(routes);

        // When
        let instruction = Instruction::GetRouteChanges {
            key: set_key,
            route_set,
        };
        let mut processor = GetRouteChanges::new(&game.tx());
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::ProcessRouteChanges(vec![RouteChange::New {
                key,
                route: route.clone()
            }])]
        );
        let routes = game.join();
        assert_eq!(
            routes,
            hashmap! {
                set_key => hashmap! {
                    key => route
                }
            }
        )
    }

    #[test]
    fn should_add_route_and_update_route_change_if_route_has_changed() {
        // Given
        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let old = Route {
            path: vec![v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(4),
            traffic: 3,
        };
        let new = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };

        let routes = hashmap! {
            set_key => hashmap! {
                key => old.clone()
            }
        };

        let route_set = hashmap! {
            key => new.clone()
        };

        let game = FnThread::new(routes);

        // When
        let instruction = Instruction::GetRouteChanges {
            key: set_key,
            route_set,
        };
        let mut processor = GetRouteChanges::new(&game.tx());
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::ProcessRouteChanges(vec![
                RouteChange::Updated {
                    key,
                    new: new.clone(),
                    old
                }
            ])]
        );
        let routes = game.join();
        assert_eq!(
            routes,
            hashmap! {
                set_key => hashmap! {
                    key => new
                }
            }
        )
    }

    #[test]
    fn should_add_no_change_instruction_if_route_is_unchanged() {
        //Given
        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };

        let route_set = hashmap! {
            key => route.clone()
        };

        let routes = hashmap! {
            set_key => route_set.clone()
        };

        let game = FnThread::new(routes);

        // When
        let instruction = Instruction::GetRouteChanges {
            key: set_key,
            route_set: route_set.clone(),
        };
        let mut processor = GetRouteChanges::new(&game.tx());
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::ProcessRouteChanges(vec![
                RouteChange::NoChange { key, route }
            ])],
        );
        let routes = game.join();
        assert_eq!(
            routes,
            hashmap! {
                set_key => route_set
            }
        )
    }

    #[test]
    fn should_remove_route_and_add_removed_route_change_if_route_is_removed() {
        // Given
        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };

        let routes = hashmap! {
            set_key => hashmap! {
                key => route.clone()
            }
        };

        let route_set = hashmap! {};

        let game = FnThread::new(routes);

        // When
        let instruction = Instruction::GetRouteChanges {
            key: set_key,
            route_set,
        };
        let mut processor = GetRouteChanges::new(&game.tx());
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::ProcessRouteChanges(vec![
                RouteChange::Removed { key, route }
            ])]
        );
        let routes = game.join();
        assert_eq!(
            routes,
            hashmap! {
                set_key => hashmap!{}
            }
        )
    }

    #[test]
    fn multiple_changes() {
        // Given
        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key_1 = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route_1 = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let key_2 = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(2, 3),
        };
        let route_2 = Route {
            path: vec![v2(1, 3), v2(2, 3)],
            start_micros: 0,
            duration: Duration::from_secs(1),
            traffic: 7,
        };

        let routes = hashmap! {};

        let route_set = hashmap! {
            key_1 => route_1.clone(),
            key_2 => route_2.clone()
        };

        let game = FnThread::new(routes);

        // When
        let instruction = Instruction::GetRouteChanges {
            key: set_key,
            route_set,
        };
        let mut processor = GetRouteChanges::new(&game.tx());
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        let actual = match state.instructions.get(0) {
            Some(Instruction::ProcessRouteChanges(actual)) => actual,
            _ => panic!("No process route changes instruction!"),
        };
        assert!(same_elements(
            &actual,
            &[
                RouteChange::New {
                    key: key_1,
                    route: route_1.clone()
                },
                RouteChange::New {
                    key: key_2,
                    route: route_2.clone()
                }
            ]
        ));
        let routes = game.join();
        assert_eq!(
            routes,
            hashmap! {
                set_key => hashmap!{
                    key_1 => route_1,
                    key_2 => route_2
                }
            }
        )
    }
}
