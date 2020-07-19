use super::*;

use crate::route::{Route, RouteKey};
use commons::grid::Grid;
use std::collections::HashSet;

pub struct GetTrafficChanges {}

impl Processor for GetTrafficChanges {
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let route_change = match instruction {
            Instruction::GetTrafficChanges(route_change) => route_change,
            _ => return state,
        };
        match route_change {
            RouteChange::New { key, route } => new(state, &key, &route),
            RouteChange::Updated { key, old, new } => updated(state, &key, &old, &new),
            RouteChange::Removed { key, route } => removed(state, &key, &route),
        }
    }
}

impl GetTrafficChanges {
    pub fn new() -> GetTrafficChanges {
        GetTrafficChanges {}
    }
}

fn new(mut state: State, key: &RouteKey, route: &Route) -> State {
    for position in route.path.iter() {
        state.traffic.mut_cell_unsafe(&position).insert(*key);
        state.instructions.push(Instruction::GetTraffic(*position));
    }
    state
}

fn updated(mut state: State, key: &RouteKey, old: &Route, new: &Route) -> State {
    let old: HashSet<&V2<usize>> = old.path.iter().collect();
    let new: HashSet<&V2<usize>> = new.path.iter().collect();

    let added = new.difference(&old).cloned();
    let removed = old.difference(&new).cloned();

    for position in added {
        state.traffic.mut_cell_unsafe(&position).insert(*key);
        state.instructions.push(Instruction::GetTraffic(*position));
    }

    for position in removed {
        state.traffic.mut_cell_unsafe(&position).remove(key);
        state.instructions.push(Instruction::GetTraffic(*position));
    }

    state
}

fn removed(mut state: State, key: &RouteKey, route: &Route) -> State {
    for position in route.path.iter() {
        state.traffic.mut_cell_unsafe(&position).remove(key);
        state.instructions.push(Instruction::GetTraffic(*position));
    }
    state
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
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
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
    fn new_route_should_append_get_traffic_instruction_for_all_positions_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };

        // When
        let state = GetTrafficChanges {}.process(state(), &Instruction::GetTrafficChanges(change));

        // Then
        assert_eq!(
            state.instructions,
            vec![
                Instruction::GetTraffic(v2(1, 3)),
                Instruction::GetTraffic(v2(2, 3)),
                Instruction::GetTraffic(v2(2, 4)),
                Instruction::GetTraffic(v2(2, 5)),
                Instruction::GetTraffic(v2(1, 5)),
            ]
        );
    }

    #[test]
    fn new_route_should_add_traffic_for_all_positions_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };

        // When
        let actual = GetTrafficChanges {}.process(state(), &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(actual.traffic, expected);
    }

    #[test]
    fn updated_route_should_append_get_traffic_instruction_for_difference() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };

        // When
        let state = GetTrafficChanges {}.process(state(), &Instruction::GetTrafficChanges(change));

        // Then
        assert!(same_elements(
            &state.instructions,
            &[
                Instruction::GetTraffic(v2(1, 4)),
                Instruction::GetTraffic(v2(2, 3)),
                Instruction::GetTraffic(v2(2, 4)),
                Instruction::GetTraffic(v2(2, 5)),
            ]
        ));
    }

    #[test]
    fn updated_route_should_remove_traffic_for_positions_not_in_new_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };
        let mut state = state();
        for position in route_1().path.iter() {
            state.traffic.mut_cell_unsafe(position).insert(key());
        }

        // When
        let actual = GetTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = traffic();
        for position in route_2().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(actual.traffic, expected);
    }

    #[test]
    fn updated_route_should_add_traffic_for_positions_not_in_old_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_2(),
            new: route_1(),
        };
        let mut state = state();
        for position in route_2().path.iter() {
            state.traffic.mut_cell_unsafe(position).insert(key());
        }

        // When
        let actual = GetTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(actual.traffic, expected);
    }

    #[test]
    fn removed_route_should_append_get_traffic_instruction_for_all_positions_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };

        // When
        let state = GetTrafficChanges {}.process(state(), &Instruction::GetTrafficChanges(change));

        // Then
        assert_eq!(
            state.instructions,
            vec![
                Instruction::GetTraffic(v2(1, 3)),
                Instruction::GetTraffic(v2(2, 3)),
                Instruction::GetTraffic(v2(2, 4)),
                Instruction::GetTraffic(v2(2, 5)),
                Instruction::GetTraffic(v2(1, 5)),
            ]
        );
    }

    #[test]
    fn removed_route_should_remove_traffic_for_all_positions_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let mut state = state();
        for position in route_1().path.iter() {
            state.traffic.mut_cell_unsafe(position).insert(key());
        }

        // When
        let actual = GetTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let expected = traffic();
        assert_eq!(actual.traffic, expected);
    }

    #[test]
    fn should_not_interfere_with_traffic_for_other_routes() {
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
        for position in route_1().path.iter() {
            state.traffic.mut_cell_unsafe(position).insert(key());
            state.traffic.mut_cell_unsafe(position).insert(key_2);
        }

        // When
        let actual = GetTrafficChanges {}.process(state, &Instruction::GetTrafficChanges(change));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key_2);
        }
        assert_eq!(actual.traffic, expected);
    }
}
