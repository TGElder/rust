use super::*;
use crate::game::traits::Routes;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

pub fn update_routes_and_get_changes(
    routes: &mut dyn Routes,
    key: &RouteSetKey,
    route_set: &RouteSet,
) -> Vec<RouteChange> {
    let mut new_and_changed = add_and_get_new_and_changed(routes, key, route_set);
    let mut removed = remove_and_get_removed(routes, key, route_set);
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
        Entry::Occupied(mut entry) if *entry.get() != route => {
            let old = entry.insert(route.clone());
            Some(RouteChange::Updated {
                key,
                old,
                new: route,
            })
        }
        Entry::Vacant(entry) => {
            entry.insert(route.clone());
            Some(RouteChange::New { key, route })
        }
        _ => None,
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
    use commons::v2;
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn should_add_route_and_return_new_route_change_if_route_is_new() {
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
        let mut route_set = HashMap::new();
        route_set.insert(key, route.clone());

        let mut routes = HashMap::new();

        // When
        let changes = update_routes_and_get_changes(&mut routes, &set_key, &route_set);

        // Then
        assert_eq!(
            changes,
            vec![RouteChange::New {
                key,
                route: route.clone()
            }]
        );
        assert_eq!(
            routes,
            vec![(set_key, vec![(key, route)].into_iter().collect())]
                .into_iter()
                .collect()
        )
    }

    #[test]
    fn should_add_route_and_return_update_route_change_if_route_has_changed() {
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
        let mut route_set = HashMap::new();
        route_set.insert(key, old.clone());
        let mut routes = HashMap::new();
        routes.insert(set_key, route_set);

        let mut route_set = HashMap::new();
        route_set.insert(key, new.clone());

        // When
        let changes = update_routes_and_get_changes(&mut routes, &set_key, &route_set);

        // Then
        assert_eq!(
            changes,
            vec![RouteChange::Updated {
                key,
                new: new.clone(),
                old
            }]
        );
        assert_eq!(
            routes,
            vec![(set_key, vec![(key, new)].into_iter().collect())]
                .into_iter()
                .collect()
        )
    }

    #[test]
    fn should_not_add_route_nor_return_anything_if_route_is_unchanged() {
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

        let mut route_set = HashMap::new();
        route_set.insert(key, route.clone());
        let mut routes = HashMap::new();
        routes.insert(set_key, route_set);

        let mut route_set = HashMap::new();
        route_set.insert(key, route.clone());

        // When
        let changes = update_routes_and_get_changes(&mut routes, &set_key, &route_set);

        // Then
        assert_eq!(changes, vec![],);
        assert_eq!(
            routes,
            vec![(set_key, vec![(key, route)].into_iter().collect())]
                .into_iter()
                .collect()
        )
    }

    #[test]
    fn should_remove_route_and_return_removed_route_change_if_route_is_removed() {
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

        let mut route_set = HashMap::new();
        route_set.insert(key, route.clone());
        let mut routes = HashMap::new();
        routes.insert(set_key, route_set);

        let route_set = HashMap::new();

        // When
        let changes = update_routes_and_get_changes(&mut routes, &set_key, &route_set);

        // Then
        assert_eq!(changes, vec![RouteChange::Removed { key, route }]);
        assert_eq!(
            routes,
            vec![(set_key, HashMap::new())].into_iter().collect()
        )
    }
}
