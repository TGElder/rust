use super::*;

use crate::route::Route;
use commons::edge::{Edge, Edges};
use std::collections::HashSet;

pub struct GetEdgeTrafficChanges {}

impl Processor for GetEdgeTrafficChanges {
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let route_change = match instruction {
            Instruction::GetTrafficChanges(route_change) => route_change,
            _ => return state,
        };
        let state = match route_change {
            RouteChange::New { route, .. } => new(state, &route),
            RouteChange::Updated { old, new, .. } => updated(state, &old, &new),
            RouteChange::Removed { route, .. } => removed(state, &route),
        };
        remove_zero_traffic_entries(state)
    }
}

impl GetEdgeTrafficChanges {
    pub fn new() -> GetEdgeTrafficChanges {
        GetEdgeTrafficChanges {}
    }
}

fn new(mut state: State, route: &Route) -> State {
    for edge in route.path.edges() {
        *state.edge_traffic.entry(edge).or_insert(0) += route.traffic;
        state.instructions.push(Instruction::GetEdgeTraffic(edge));
    }
    state
}

fn updated(mut state: State, old: &Route, new: &Route) -> State {
    let old_edges: HashSet<Edge> = old.path.edges().collect();
    let new_edges: HashSet<Edge> = new.path.edges().collect();

    let added = new_edges.difference(&old_edges).cloned();
    let removed = old_edges.difference(&new_edges).cloned();

    for edge in added {
        *state.edge_traffic.entry(edge).or_insert(0) += new.traffic;
        state.instructions.push(Instruction::GetEdgeTraffic(edge));
    }

    for edge in removed {
        *state.edge_traffic.entry(edge).or_insert(0) -= old.traffic;
        state.instructions.push(Instruction::GetEdgeTraffic(edge));
    }

    state
}

fn removed(mut state: State, route: &Route) -> State {
    for edge in route.path.edges() {
        *state.edge_traffic.entry(edge).or_insert(0) -= route.traffic;
        state.instructions.push(Instruction::GetEdgeTraffic(edge));
    }
    state
}

fn remove_zero_traffic_entries(mut state: State) -> State {
    state.edge_traffic.retain(|_, traffic| *traffic > 0);
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::route::RouteKey;
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
    fn new_route_should_append_get_edge_traffic_instruction_for_all_edges_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };

        // When
        let state =
            GetEdgeTrafficChanges {}.process(state(), &Instruction::GetTrafficChanges(change));

        // Then
        assert_eq!(
            state.instructions,
            vec![
                Instruction::GetEdgeTraffic(Edge::new(v2(1, 3), v2(2, 3))),
                Instruction::GetEdgeTraffic(Edge::new(v2(2, 3), v2(2, 4))),
                Instruction::GetEdgeTraffic(Edge::new(v2(2, 4), v2(2, 5))),
                Instruction::GetEdgeTraffic(Edge::new(v2(2, 5), v2(1, 5))),
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

        // When
        let actual =
            GetEdgeTrafficChanges {}.process(state(), &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, 3);
        }
        assert_eq!(actual.edge_traffic, expected);
    }

    #[test]
    fn updated_route_should_append_get_edge_traffic_instruction_for_difference() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, 3);
        }

        // When
        let state =
            GetEdgeTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        assert!(same_elements(
            &state.instructions,
            &[
                Instruction::GetEdgeTraffic(Edge::new(v2(1, 3), v2(2, 3))),
                Instruction::GetEdgeTraffic(Edge::new(v2(2, 3), v2(2, 4))),
                Instruction::GetEdgeTraffic(Edge::new(v2(1, 3), v2(1, 4))),
                Instruction::GetEdgeTraffic(Edge::new(v2(1, 4), v2(2, 4))),
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
            state.edge_traffic.insert(edge, 3);
        }

        // When
        let actual =
            GetEdgeTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = hashmap! {};
        for edge in route_2().path.edges() {
            expected.insert(edge, 3);
        }
        assert_eq!(actual.edge_traffic, expected);
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
            state.edge_traffic.insert(edge, 3);
        }

        // When
        let actual =
            GetEdgeTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, 3);
        }
        assert_eq!(actual.edge_traffic, expected);
    }

    #[test]
    fn removed_route_should_append_get_edge_traffic_instruction_for_all_edges_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, 3);
        }

        // When
        let state =
            GetEdgeTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        assert_eq!(
            state.instructions,
            vec![
                Instruction::GetEdgeTraffic(Edge::new(v2(1, 3), v2(2, 3))),
                Instruction::GetEdgeTraffic(Edge::new(v2(2, 3), v2(2, 4))),
                Instruction::GetEdgeTraffic(Edge::new(v2(2, 4), v2(2, 5))),
                Instruction::GetEdgeTraffic(Edge::new(v2(2, 5), v2(1, 5))),
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
            state.edge_traffic.insert(edge, 3);
        }

        // When
        let actual =
            GetEdgeTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let expected = hashmap! {};
        assert_eq!(actual.edge_traffic, expected);
    }

    #[test]
    fn should_retain_traffic_added_by_other_route_when_adding_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, 2);
        }

        // When
        let actual =
            GetEdgeTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, 5);
        }
        assert_eq!(actual.edge_traffic, expected);
    }

    #[test]
    fn should_retain_traffic_added_by_other_route_when_removing_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let mut state = state();
        for edge in route_1().path.edges() {
            state.edge_traffic.insert(edge, 5);
        }

        // When
        let actual =
            GetEdgeTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, 2);
        }
        assert_eq!(actual.edge_traffic, expected);
    }
}
