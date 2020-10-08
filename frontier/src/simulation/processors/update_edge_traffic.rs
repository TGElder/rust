use super::*;

use crate::route::{Route, RouteKey};
use commons::edge::{Edge, Edges};
use std::collections::HashSet;

pub struct UpdateEdgeTraffic {}

#[async_trait]
impl Processor for UpdateEdgeTraffic {
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let route_changes = match instruction {
            Instruction::ProcessRouteChanges(route_changes) => route_changes,
            _ => return state,
        };
        let changed_edges = update_all_edge_traffic_and_get_changes(&mut state, route_changes);
        state
            .instructions
            .push(Instruction::RefreshEdges(changed_edges));
        state
    }
}

impl UpdateEdgeTraffic {
    pub fn new() -> UpdateEdgeTraffic {
        UpdateEdgeTraffic {}
    }
}

pub fn update_all_edge_traffic_and_get_changes(
    state: &mut State,
    route_changes: &[RouteChange],
) -> HashSet<Edge> {
    route_changes
        .iter()
        .flat_map(|route_change| update_edge_traffic_and_get_changes(state, route_change))
        .collect()
}

fn update_edge_traffic_and_get_changes(state: &mut State, route_change: &RouteChange) -> Vec<Edge> {
    let out = match route_change {
        RouteChange::New { key, route } => new(state, &key, &route),
        RouteChange::Updated { key, old, new } => updated(state, &key, &old, &new),
        RouteChange::Removed { key, route } => removed(state, &key, &route),
        RouteChange::NoChange { route, .. } => no_change(&route),
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
    let union = new_edges.union(&old_edges).cloned();

    for edge in added {
        state
            .edge_traffic
            .entry(edge)
            .or_insert_with(HashSet::new)
            .insert(*key);
    }

    for edge in removed {
        state
            .edge_traffic
            .entry(edge)
            .or_insert_with(HashSet::new)
            .remove(key);
    }

    for edge in union {
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

fn no_change(route: &Route) -> Vec<Edge> {
    route.path.edges().collect()
}

fn remove_zero_traffic_entries(state: &mut State) {
    state.edge_traffic.retain(|_, traffic| !traffic.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use commons::index2d::Vec2D;
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
    fn new_route_should_refresh_all_edges_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };

        // When
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state(), &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshEdges(hashset! {
                Edge::new(v2(1, 3), v2(2, 3)),
                Edge::new(v2(2, 3), v2(2, 4)),
                Edge::new(v2(2, 4), v2(2, 5)),
                Edge::new(v2(2, 5), v2(1, 5)),
            })]
        );
    }

    #[test]
    fn new_route_should_add_edge_traffic_for_all_edges_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let state = state();

        // When
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn updated_route_should_refresh_edges_in_old_and_new() {
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
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshEdges(hashset! {
             Edge::new(v2(1, 3), v2(2, 3)),
             Edge::new(v2(2, 3), v2(2, 4)),
             Edge::new(v2(2, 4), v2(2, 5)),
             Edge::new(v2(2, 5), v2(1, 5)),
             Edge::new(v2(1, 3), v2(1, 4)),
             Edge::new(v2(1, 4), v2(2, 4)),
            })]
        );
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
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

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
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn removed_route_should_refresh_all_edges_in_route() {
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
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshEdges(hashset! {
             Edge::new(v2(1, 3), v2(2, 3)),
             Edge::new(v2(2, 3), v2(2, 4)),
             Edge::new(v2(2, 4), v2(2, 5)),
             Edge::new(v2(2, 5), v2(1, 5)),
            })]
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
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let expected = hashmap! {};
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn no_change_route_should_refresh_all_edges_in_route() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };
        let state = state();

        // When
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshEdges(hashset! {
             Edge::new(v2(1, 3), v2(2, 3)),
             Edge::new(v2(2, 3), v2(2, 4)),
             Edge::new(v2(2, 4), v2(2, 5)),
             Edge::new(v2(2, 5), v2(1, 5)),
            })]
        );
    }

    #[test]
    fn no_change_route_should_not_change_edge_traffic() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };
        let state = state();

        // When
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(state.edge_traffic, hashmap! {},);
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
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

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
        let state = block_on(
            UpdateEdgeTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key_2});
        }
        assert_eq!(state.edge_traffic, expected);
    }

    #[test]
    fn multiple_changes() {
        // Given
        let change_1 = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let change_2 = RouteChange::New {
            key: RouteKey {
                settlement: v2(1, 3),
                resource: Resource::Coal,
                destination: v2(1, 5),
            },
            route: route_2(),
        };

        // When
        let state = block_on(UpdateEdgeTraffic::new().process(
            state(),
            &Instruction::ProcessRouteChanges(vec![change_1, change_2]),
        ));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshEdges(hashset! {
               Edge::new(v2(1, 3), v2(2, 3)),
               Edge::new(v2(2, 3), v2(2, 4)),
               Edge::new(v2(2, 4), v2(2, 5)),
               Edge::new(v2(2, 5), v2(1, 5)),
               Edge::new(v2(1, 3), v2(1, 4)),
               Edge::new(v2(1, 4), v2(2, 4)),
            })]
        );
    }
}
