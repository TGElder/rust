use super::*;

use crate::route::{Route, RouteKey};
use commons::edge::{Edge, Edges};
use std::collections::HashSet;

pub fn update_edge_traffic_and_get_changes(
    state: &mut State,
    route_change: &RouteChange,
) -> Vec<Edge> {
    let out = match route_change {
        RouteChange::New { key, route } => new(state, &key, &route),
        RouteChange::Updated { key, old, new } => updated(state, &key, &old, &new),
        RouteChange::Removed { key, route } => removed(state, &key, &route),
    };
    remove_zero_traffic_entries(state);
    out
}

fn new(state: &mut State, key: &RouteKey, route: &Route) -> Vec<Edge> {
    let mut out = vec![];
    for edge in route.path.edges() {
        state
            .edge_traffic
            .entry(edge)
            .or_insert_with(HashSet::new)
            .insert(*key);
        out.push(edge);
    }
    out
}

fn updated(state: &mut State, key: &RouteKey, old: &Route, new: &Route) -> Vec<Edge> {
    let mut out = vec![];
    let old_edges: HashSet<Edge> = old.path.edges().collect();
    let new_edges: HashSet<Edge> = new.path.edges().collect();

    let added = new_edges.difference(&old_edges).cloned();
    let removed = old_edges.difference(&new_edges).cloned();

    for edge in added {
        state
            .edge_traffic
            .entry(edge)
            .or_insert_with(HashSet::new)
            .insert(*key);
        out.push(edge);
    }

    for edge in removed {
        state
            .edge_traffic
            .entry(edge)
            .or_insert_with(HashSet::new)
            .remove(key);
        out.push(edge);
    }

    out
}

fn removed(state: &mut State, key: &RouteKey, route: &Route) -> Vec<Edge> {
    let mut out = vec![];
    for edge in route.path.edges() {
        state
            .edge_traffic
            .entry(edge)
            .or_insert_with(HashSet::new)
            .remove(key);
        out.push(edge);
    }
    out
}

fn remove_zero_traffic_entries(state: &mut State) {
    state.edge_traffic.retain(|_, traffic| !traffic.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::world::Resource;
    use commons::index2d::Vec2D;
    use commons::same_elements;
    use commons::v2;
    use std::collections::HashSet;
    use std::time::Duration;

    fn key() -> RouteKey {
        RouteKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
            destination: v2(1, 5),
        }
    }

    fn route_1() -> Route {
        Route {
            path: vec![v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(4),
            traffic: 3,
        }
    }

    fn route_2() -> Route {
        Route {
            path: vec![v2(1, 3), v2(1, 4), v2(2, 4), v2(2, 5), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        }
    }

    fn traffic() -> Traffic {
        Vec2D::new(6, 6, HashSet::with_capacity(0))
    }

    fn state() -> State {
        State {
            traffic: traffic(),
            ..State::default()
        }
    }

    #[test]
    fn new_route_should_return_all_edges_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };

        // When
        let edges = update_edge_traffic_and_get_changes(&mut state(), &change);

        // Then
        assert_eq!(
            edges,
            vec![
                Edge::new(v2(1, 3), v2(2, 3)),
                Edge::new(v2(2, 3), v2(2, 4)),
                Edge::new(v2(2, 4), v2(2, 5)),
                Edge::new(v2(2, 5), v2(1, 5)),
            ]
        );
    }

    #[test]
    fn new_route_should_add_edge_traffic_for_all_edges_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let mut state = state();

        // When
        update_edge_traffic_and_get_changes(&mut state, &change);

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn updated_route_should_return_different_edges() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, hashset! {key()});
        }

        // When
        let edges = update_edge_traffic_and_get_changes(&mut state, &change);

        // Then
        assert!(same_elements(
            &edges,
            &[
                Edge::new(v2(1, 3), v2(2, 3)),
                Edge::new(v2(2, 3), v2(2, 4)),
                Edge::new(v2(1, 3), v2(1, 4)),
                Edge::new(v2(1, 4), v2(2, 4)),
            ]
        ));
    }

    #[test]
    fn updated_route_should_remove_edge_traffic_for_edges_not_in_new_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, hashset! {key()});
        }

        // When
        update_edge_traffic_and_get_changes(&mut state, &change);

        // Then
        let mut expected = hashmap! {};
        for edge in route_2().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn updated_route_should_add_edge_traffic_for_edges_not_in_old_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_2(),
            new: route_1(),
        };
        let mut state = state();
        for edge in route_2().path.edges() {
            state.edge_traffic.insert(edge, hashset! {key()});
        }

        // When
        update_edge_traffic_and_get_changes(&mut state, &change);

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn removed_route_should_return_all_edges_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, hashset! {key()});
        }

        // When
        let edges = update_edge_traffic_and_get_changes(&mut state, &change);

        // Then
        assert_eq!(
            edges,
            vec![
                Edge::new(v2(1, 3), v2(2, 3)),
                Edge::new(v2(2, 3), v2(2, 4)),
                Edge::new(v2(2, 4), v2(2, 5)),
                Edge::new(v2(2, 5), v2(1, 5)),
            ]
        );
    }

    #[test]
    fn removed_route_should_remove_edge_traffic_for_all_edges_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, hashset! {key()});
        }

        // When
        update_edge_traffic_and_get_changes(&mut state, &change);

        // Then
        let expected = hashmap! {};
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn should_retain_traffic_added_by_other_route_when_adding_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let key_2 = RouteKey {
            settlement: v2(1, 4),
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, hashset! {key_2});
        }

        // When
        update_edge_traffic_and_get_changes(&mut state, &change);

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key_2, key()});
        }
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn should_retain_traffic_added_by_other_route_when_removing_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let key_2 = RouteKey {
            settlement: v2(1, 4),
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, hashset! {key_2, key()});
        }

        // When
        update_edge_traffic_and_get_changes(&mut state, &change);

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key_2});
        }
        assert_eq!(state.edge_traffic, expected);
    }
}
