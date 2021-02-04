use super::*;

use crate::route::{Route, RouteKey};
use commons::grid::Grid;
use std::collections::HashSet;

pub struct UpdatePositionTraffic {}

#[async_trait]
impl Processor for UpdatePositionTraffic {
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let route_changes = match instruction {
            Instruction::ProcessRouteChanges(route_changes) => route_changes,
            _ => return state,
        };
        let changed_positions =
            update_all_position_traffic_and_get_changes(&mut state, route_changes);
        state
            .instructions
            .push(Instruction::RefreshPositions(changed_positions));
        state
    }
}

impl UpdatePositionTraffic {
    pub fn new() -> UpdatePositionTraffic {
        UpdatePositionTraffic {}
    }
}

fn update_all_position_traffic_and_get_changes(
    state: &mut State,
    route_changes: &[RouteChange],
) -> HashSet<V2<usize>> {
    route_changes
        .iter()
        .flat_map(|route_change| update_position_traffic_and_get_changes(state, route_change))
        .collect()
}

fn update_position_traffic_and_get_changes(
    state: &mut State,
    route_change: &RouteChange,
) -> Vec<V2<usize>> {
    match route_change {
        RouteChange::New { key, route } => new(state, &key, &route),
        RouteChange::Updated { key, old, new } => updated(state, &key, &old, &new),
        RouteChange::Removed { key, route } => removed(state, &key, &route),
        RouteChange::NoChange { route, .. } => no_change(&route),
    }
}

fn new(state: &mut State, key: &RouteKey, route: &Route) -> Vec<V2<usize>> {
    let mut out = vec![];
    for position in route.path.iter() {
        state.traffic.mut_cell_unsafe(&position).insert(*key);
        out.push(*position);
    }
    out
}

fn updated(state: &mut State, key: &RouteKey, old: &Route, new: &Route) -> Vec<V2<usize>> {
    let mut out = vec![];

    let old: HashSet<&V2<usize>> = old.path.iter().collect();
    let new: HashSet<&V2<usize>> = new.path.iter().collect();

    let added = new.difference(&old).cloned();
    let removed = old.difference(&new).cloned();
    let union = new.union(&old).cloned();

    for position in added {
        state.traffic.mut_cell_unsafe(&position).insert(*key);
    }

    for position in removed {
        state.traffic.mut_cell_unsafe(&position).remove(key);
    }

    for position in union {
        out.push(*position);
    }

    out
}

fn removed(state: &mut State, key: &RouteKey, route: &Route) -> Vec<V2<usize>> {
    let mut out = vec![];
    for position in route.path.iter() {
        state.traffic.mut_cell_unsafe(&position).remove(key);
        out.push(*position);
    }
    out
}

fn no_change(route: &Route) -> Vec<V2<usize>> {
    route.path.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use commons::index2d::Vec2D;
    use commons::v2;
    use futures::executor::block_on;
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
    fn new_route_should_refresh_all_positions_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };

        // When
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state(), &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshPositions(
                hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
            )]
        );
    }

    #[test]
    fn new_route_should_add_traffic_for_all_positions_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let state = state();

        // When
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(state.traffic, expected);
    }

    #[test]
    fn updated_route_should_refresh_positions_from_old_and_new_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };

        // When
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state(), &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshPositions(
                hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5), v2(1, 4)}
            )]
        );
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
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let mut expected = traffic();
        for position in route_2().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(state.traffic, expected);
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
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(state.traffic, expected);
    }

    #[test]
    fn removed_route_should_refresh_all_positions_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };

        // When
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state(), &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshPositions(
                hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
            )]
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
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let expected = traffic();
        assert_eq!(state.traffic, expected);
    }

    #[test]
    fn no_change_route_should_refresh_all_positions_in_route() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };

        // When
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state(), &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshPositions(
                hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
            )]
        );
    }

    #[test]
    fn no_change_route_should_not_change_traffic() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };
        let state = state();

        // When
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        assert_eq!(state.traffic, traffic(),);
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
        for position in route_1().path.iter() {
            state.traffic.mut_cell_unsafe(position).insert(key_2);
        }

        // When
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
            expected.mut_cell_unsafe(position).insert(key_2);
        }
        assert_eq!(state.traffic, expected);
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
        for position in route_1().path.iter() {
            state.traffic.mut_cell_unsafe(position).insert(key());
            state.traffic.mut_cell_unsafe(position).insert(key_2);
        }

        // When
        let state = block_on(
            UpdatePositionTraffic::new()
                .process(state, &Instruction::ProcessRouteChanges(vec![change])),
        );

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key_2);
        }
        assert_eq!(state.traffic, expected);
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
        let state = block_on(UpdatePositionTraffic::new().process(
            state(),
            &Instruction::ProcessRouteChanges(vec![change_1, change_2]),
        ));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshPositions(
                hashset! {v2(1, 3), v2(1, 4), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
            )]
        );
    }
}