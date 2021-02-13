use crate::route::{Route, RouteKey, RouteSet, RouteSetKey, Routes};
use crate::simulation::settlement::instruction::RouteChange;
use crate::simulation::settlement::UpdateSettlement;
use crate::traits::SendRoutes;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

impl<T> UpdateSettlement<T>
where
    T: SendRoutes,
{
    pub async fn update_routes_and_get_changes(
        &self,
        key: RouteSetKey,
        route_set: RouteSet,
    ) -> Vec<RouteChange> {
        self.tx
            .send_routes(move |routes| update_routes_and_get_changes(routes, key, route_set))
            .await
    }
}

pub fn update_routes_and_get_changes(
    routes: &mut Routes,
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
    routes: &mut Routes,
    set_key: &RouteSetKey,
    route_set: &RouteSet,
) -> Vec<RouteChange> {
    route_set
        .iter()
        .flat_map(move |(key, route)| add_and_get_change(routes, *set_key, *key, route.clone()))
        .collect()
}

fn add_and_get_change(
    routes: &mut Routes,
    set_key: RouteSetKey,
    key: RouteKey,
    route: Route,
) -> Option<RouteChange> {
    let route_set = routes.entry(set_key).or_insert_with(HashMap::new);
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
    routes: &mut Routes,
    set_key: &RouteSetKey,
    new_route_set: &RouteSet,
) -> Vec<RouteChange> {
    let old_route_set = routes.entry(*set_key).or_insert_with(HashMap::new);
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
    use commons::async_trait::async_trait;
    use commons::same_elements;
    use commons::v2;
    use futures::executor::block_on;
    use std::sync::Mutex;
    use std::time::Duration;

    #[async_trait]
    impl SendRoutes for Mutex<Routes> {
        async fn send_routes<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut crate::route::Routes) -> O + Send + 'static,
        {
            function(&mut self.lock().unwrap())
        }
    }

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

        let routes = Mutex::new(routes);

        // When
        let sim = UpdateSettlement::new(routes);
        let route_changes = block_on(sim.update_routes_and_get_changes(set_key, route_set));

        // Then
        assert_eq!(
            route_changes,
            vec![RouteChange::New {
                key,
                route: route.clone()
            }]
        );
        let routes = sim.tx.lock().unwrap();
        assert_eq!(
            *routes,
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

        let routes = Mutex::new(routes);

        // When
        let sim = UpdateSettlement::new(routes);
        let route_changes = block_on(sim.update_routes_and_get_changes(set_key, route_set));

        // Then
        assert_eq!(
            route_changes,
            vec![RouteChange::Updated {
                key,
                new: new.clone(),
                old
            }]
        );
        let routes = sim.tx.lock().unwrap();
        assert_eq!(
            *routes,
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

        let routes = Mutex::new(routes);

        // When
        let sim = UpdateSettlement::new(routes);
        let route_changes = block_on(sim.update_routes_and_get_changes(set_key, route_set.clone()));

        // Then
        assert_eq!(route_changes, vec![RouteChange::NoChange { key, route }]);

        let routes = sim.tx.lock().unwrap();
        assert_eq!(
            *routes,
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

        let routes = Mutex::new(routes);

        // When
        let sim = UpdateSettlement::new(routes);
        let route_changes = block_on(sim.update_routes_and_get_changes(set_key, route_set));

        // Then
        assert_eq!(route_changes, vec![RouteChange::Removed { key, route }]);

        let routes = sim.tx.lock().unwrap();
        assert_eq!(
            *routes,
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

        let routes = Mutex::new(routes);

        // When
        let sim = UpdateSettlement::new(routes);
        let route_changes = block_on(sim.update_routes_and_get_changes(set_key, route_set));

        // Then
        assert!(same_elements(
            &route_changes,
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
        let routes = sim.tx.lock().unwrap();
        assert_eq!(
            *routes,
            hashmap! {
                set_key => hashmap!{
                    key_1 => route_1,
                    key_2 => route_2
                }
            }
        )
    }
}
