use crate::travel_duration::*;
use commons::manhattan::ManhattanDistance;
use commons::{index2d::*, V2};
use network::ClosestTargetResult as NetworkClosestTargetResult;
use network::Edge as NetworkEdge;
use network::Network;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct Pathfinder<T>
where
    T: TravelDuration,
{
    index: Index2D,
    travel_duration: Arc<T>,
    network: Network,
}

impl<T> Pathfinder<T>
where
    T: TravelDuration,
{
    pub fn new(width: usize, height: usize, travel_duration: Arc<T>) -> Pathfinder<T> {
        Pathfinder {
            index: Index2D::new(width, height),
            travel_duration,
            network: Network::new(width * height * 2, &[]),
        }
    }

    pub fn travel_duration(&self) -> &Arc<T> {
        &self.travel_duration
    }

    fn get_network_index(&self, position: &TravelPosition) -> usize {
        self.index.get_index(&position.into()).unwrap()
            + (self.index.indices() * position.mode as usize)
    }

    fn get_network_indices(&self, positions: &[TravelPosition]) -> Vec<usize> {
        positions
            .iter()
            .map(|position| self.get_network_index(position))
            .collect()
    }

    fn get_position_from_network_index(
        &self,
        network_index: usize,
    ) -> Result<TravelPosition, IndexOutOfBounds> {
        let indices = self.index.indices();
        if network_index < indices {
            self.index
                .get_position(network_index)
                .map(|position| land(position.x as u16, position.y as u16))
        } else {
            self.index
                .get_position(network_index - indices)
                .map(|position| water(position.x as u16, position.y as u16))
        }
    }

    fn get_positions_from_network_indices(&self, network_indices: &[usize]) -> Vec<TravelPosition> {
        network_indices
            .iter()
            .flat_map(|index| self.get_position_from_network_index(*index))
            .collect()
    }

    pub fn remove_edge(&mut self, from: &TravelPosition, to: &TravelPosition) {
        self.network
            .remove_edges(self.get_network_index(from), self.get_network_index(to));
    }

    pub fn set_edge_duration(
        &mut self,
        from: &TravelPosition,
        to: &TravelPosition,
        duration: &Duration,
    ) {
        self.remove_edge(from, to);
        let network_edge = NetworkEdge::new(
            self.get_network_index(from),
            self.get_network_index(to),
            self.travel_duration.get_cost_from_duration_u8(duration),
        );
        self.network.add_edge(&network_edge);
    }

    pub fn manhattan_distance(&self, to: &[TravelPosition]) -> impl Fn(usize) -> u32 {
        let to = to.iter().map(|to| to.into()).collect::<Vec<V2<usize>>>();
        let index = self.index;
        let indices = self.index.indices();
        let minimum_duration = self.travel_duration.min_duration();
        let minimum_cost = self
            .travel_duration
            .get_cost_from_duration_u8(&minimum_duration) as u32;
        move |from| {
            let from = index.get_position(from % indices).unwrap();
            to.iter()
                .map(|to| from.manhattan_distance(&to) as u32 * minimum_cost)
                .min()
                .unwrap()
        }
    }

    pub fn find_path(
        &self,
        from: &[TravelPosition],
        to: &[TravelPosition],
    ) -> Option<Vec<TravelPosition>> {
        let to_indices = &self.get_network_indices(to);
        if to_indices.is_empty() {
            return None;
        }
        let from_indices = &self.get_network_indices(from);
        if from_indices.is_empty() {
            return None;
        }
        let path = self.network.find_path(
            &from_indices,
            &to_indices,
            None,
            &self.manhattan_distance(to),
        );
        match path {
            Some(ref path) if path.is_empty() => None,
            Some(ref path) => {
                let mut out = vec![self.get_position_from_network_index(path[0].from).unwrap()];
                for edge in path {
                    out.push(self.get_position_from_network_index(edge.to).unwrap());
                }
                Some(out)
            }
            None => None,
        }
    }

    pub fn in_bounds(&self, position: &TravelPosition) -> bool {
        self.index.get_index(&position.into()).is_ok()
    }

    pub fn positions_within(
        &self,
        positions: &[TravelPosition],
        duration: &Duration,
    ) -> HashMap<TravelPosition, Duration> {
        let indices = self.get_network_indices(positions);
        let max_cost = self.travel_duration.get_cost_from_duration(&duration);
        self.network
            .nodes_within(&indices, max_cost)
            .into_iter()
            .flat_map(|result| {
                let position = self.get_position_from_network_index(result.index);
                match position {
                    Ok(position) => Some((
                        position,
                        self.travel_duration.get_duration_from_cost(result.cost),
                    )),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn init_targets(&mut self, name: String) {
        self.network.init_targets(name);
    }

    pub fn load_target(&mut self, name: &str, position: &TravelPosition, target: bool) {
        self.network
            .load_target(name, self.get_network_index(position), target)
    }

    pub fn closest_targets(
        &self,
        positions: &[TravelPosition],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult> {
        let indices = self.get_network_indices(positions);
        self.network
            .closest_loaded_targets(&indices, targets, n_closest)
            .drain(..)
            .map(|result| self.as_closest_target_result(result))
            .collect()
    }

    fn as_closest_target_result(&self, result: NetworkClosestTargetResult) -> ClosestTargetResult {
        ClosestTargetResult {
            position: self.get_position_from_network_index(result.node).unwrap(),
            path: self.get_positions_from_network_indices(&result.path),
            duration: self.travel_duration.get_duration_from_cost(result.cost),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClosestTargetResult {
    pub position: TravelPosition,
    pub path: Vec<TravelPosition>,
    pub duration: Duration,
}

#[cfg(test)]
mod tests {

    use crate::world::World;

    use super::*;
    use commons::edge::Edge;
    use commons::grid::Grid;
    use commons::{v2, M, V2};
    use isometric::cell_traits::*;
    use std::time::Duration;

    struct TestTravelDuration {
        max: Duration,
    }

    impl TravelDuration for TestTravelDuration {
        // TODO pathfinding tests should not rely on this
        fn get_duration(
            &self,
            world: &World,
            from: &V2<usize>,
            to: &V2<usize>,
        ) -> Option<Duration> {
            match world.get_cell(to) {
                Some(cell) => {
                    let elevation = cell.elevation();
                    if world.is_road(&Edge::new(*from, *to)) {
                        return Some(Duration::from_millis(1));
                    } else if elevation != 0.0 {
                        return Some(Duration::from_millis(elevation as u64));
                    }
                }
                _ => return None,
            }
            None
        }

        fn min_duration(&self) -> Duration {
            Duration::from_millis(1)
        }

        fn max_duration(&self) -> Duration {
            self.max
        }
    }

    fn travel_duration() -> TestTravelDuration {
        TestTravelDuration {
            max: Duration::from_millis(4),
        }
    }

    #[rustfmt::skip]
    fn world() -> World {
        World::new(
            M::from_vec(3, 3, 
            vec![
                4.0, 2.0, 0.0,
                3.0, 3.0, 2.0,
                2.0, 3.0, 4.0]
            ),
            0.5,
        )
    }

    fn pathfinder() -> Pathfinder<TestTravelDuration> {
        let world = &world();
        let mut out = Pathfinder::new(world.width(), world.height(), Arc::new(travel_duration()));

        let travel_duration = travel_duration();
        for x in 0..world.width() {
            for y in 0..world.height() {
                for EdgeDuration { from, to, duration } in
                    travel_duration.get_durations_for_position(world, v2(x, y))
                {
                    if let Some(duration) = duration {
                        out.set_edge_duration(&from, &to, &duration)
                    }
                }
            }
        }
        out
    }

    #[test]
    fn test_get_network_index() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.get_network_index(&land(0, 0)), 0);
        assert_eq!(pathfinder.get_network_index(&land(1, 0)), 1);
        assert_eq!(pathfinder.get_network_index(&land(2, 0)), 2);
        assert_eq!(pathfinder.get_network_index(&land(0, 1)), 3);
        assert_eq!(pathfinder.get_network_index(&land(1, 1)), 4);
        assert_eq!(pathfinder.get_network_index(&land(2, 1)), 5);
        assert_eq!(pathfinder.get_network_index(&land(0, 2)), 6);
        assert_eq!(pathfinder.get_network_index(&land(1, 2)), 7);
        assert_eq!(pathfinder.get_network_index(&land(2, 2)), 8);
        assert_eq!(pathfinder.get_network_index(&water(0, 0)), 9);
        assert_eq!(pathfinder.get_network_index(&water(1, 0)), 10);
        assert_eq!(pathfinder.get_network_index(&water(2, 0)), 11);
        assert_eq!(pathfinder.get_network_index(&water(0, 1)), 12);
        assert_eq!(pathfinder.get_network_index(&water(1, 1)), 13);
        assert_eq!(pathfinder.get_network_index(&water(2, 1)), 14);
        assert_eq!(pathfinder.get_network_index(&water(0, 2)), 15);
        assert_eq!(pathfinder.get_network_index(&water(1, 2)), 16);
        assert_eq!(pathfinder.get_network_index(&water(2, 2)), 17);
    }

    #[test]
    fn test_get_network_indices() {
        let pathfinder = pathfinder();
        let positions = [
            land(0, 0),
            land(1, 0),
            land(2, 0),
            land(0, 1),
            land(1, 1),
            land(2, 1),
            land(0, 2),
            land(1, 2),
            land(2, 2),
            water(0, 0),
            water(1, 0),
            water(2, 0),
            water(0, 1),
            water(1, 1),
            water(2, 1),
            water(0, 2),
            water(1, 2),
            water(2, 2),
        ];
        let actual = pathfinder.get_network_indices(&positions);
        assert_eq!(
            actual,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
        );
    }

    #[test]
    #[should_panic]
    fn test_get_network_indices_out_of_bounds() {
        let pathfinder = pathfinder();
        let positions = [land(3, 0)];
        pathfinder.get_network_indices(&positions);
    }

    #[test]
    fn test_get_network_position() {
        let pathfinder = pathfinder();
        assert_eq!(
            pathfinder.get_position_from_network_index(0),
            Ok(land(0, 0))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(1),
            Ok(land(1, 0))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(2),
            Ok(land(2, 0))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(3),
            Ok(land(0, 1))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(4),
            Ok(land(1, 1))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(5),
            Ok(land(2, 1))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(6),
            Ok(land(0, 2))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(7),
            Ok(land(1, 2))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(8),
            Ok(land(2, 2))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(9),
            Ok(water(0, 0))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(10),
            Ok(water(1, 0))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(11),
            Ok(water(2, 0))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(12),
            Ok(water(0, 1))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(13),
            Ok(water(1, 1))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(14),
            Ok(water(2, 1))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(15),
            Ok(water(0, 2))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(16),
            Ok(water(1, 2))
        );
        assert_eq!(
            pathfinder.get_position_from_network_index(17),
            Ok(water(2, 2))
        );
    }

    #[test]
    fn test_get_positions_from_network_indices() {
        let pathfinder = pathfinder();
        let indices = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
        let actual = pathfinder.get_positions_from_network_indices(&indices);
        assert_eq!(
            actual,
            vec![
                land(0, 0),
                land(1, 0),
                land(2, 0),
                land(0, 1),
                land(1, 1),
                land(2, 1),
                land(0, 2),
                land(1, 2),
                land(2, 2),
                water(0, 0),
                water(1, 0),
                water(2, 0),
                water(0, 1),
                water(1, 1),
                water(2, 1),
                water(0, 2),
                water(1, 2),
                water(2, 2),
            ]
        );
    }

    #[test]
    fn test_get_positions_from_network_indices_out_of_bounds() {
        let pathfinder = pathfinder();
        let indices = [18];
        let actual = pathfinder.get_positions_from_network_indices(&indices);
        assert!(actual.is_empty());
    }

    #[test]
    fn test_find_path() {
        let pathfinder = pathfinder();
        assert_eq!(
            pathfinder.find_path(&[land(2, 2)], &[land(1, 0)]),
            Some(vec![land(2, 2), land(2, 1), land(1, 1), land(1, 0),])
        );
    }

    #[test]
    fn test_find_path_impossible() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.find_path(&[land(2, 2)], &[land(2, 0)]), None);
    }

    #[test]
    fn test_find_path_length_0() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.find_path(&[land(2, 2)], &[land(2, 2)]), None);
    }

    #[test]
    fn test_find_path_multiple_from() {
        let pathfinder = pathfinder();
        assert_eq!(
            pathfinder.find_path(&[land(0, 0), land(1, 0)], &[land(1, 2)]),
            Some(vec![land(1, 0), land(1, 1), land(1, 2)])
        );
    }

    #[test]
    fn test_find_path_multiple_to() {
        let pathfinder = pathfinder();
        assert_eq!(
            pathfinder.find_path(&[land(0, 0)], &[land(2, 1), land(0, 2)]),
            Some(vec![land(0, 0), land(0, 1), land(0, 2)])
        );
    }

    #[test]
    fn test_set_edge_duration() {
        // Given
        let mut pathfinder = pathfinder();

        // When
        pathfinder.set_edge_duration(&land(0, 0), &land(1, 0), &Duration::from_millis(0));

        // Then
        assert_eq!(
            pathfinder
                .network
                .get_out(&0)
                .iter()
                .find(|edge| edge.to == 1),
            Some(&NetworkEdge {
                from: 0,
                to: 1,
                cost: 0
            })
        );
        assert_eq!(
            pathfinder
                .network
                .get_in(&1)
                .iter()
                .find(|edge| edge.from == 0),
            Some(&NetworkEdge {
                from: 0,
                to: 1,
                cost: 0
            })
        );
    }

    #[test]
    fn test_remove_edge() {
        // Given
        let mut pathfinder = pathfinder();

        // When
        pathfinder.set_edge_duration(&land(0, 0), &land(1, 0), &Duration::from_millis(0));
        pathfinder.remove_edge(&land(0, 0), &land(1, 0));

        // Then
        assert_eq!(
            pathfinder
                .network
                .get_out(&0)
                .iter()
                .find(|edge| edge.to == 1),
            None
        );
        assert_eq!(
            pathfinder
                .network
                .get_in(&1)
                .iter()
                .find(|edge| edge.from == 0),
            None
        );
    }

    #[test]
    fn test_positions_within() {
        let pathfinder = pathfinder();
        let actual = pathfinder.positions_within(&[land(0, 0)], &Duration::from_millis(5));
        let expected = [
            (land(0, 0), Duration::from_millis(0)),
            (land(1, 0), Duration::from_millis(2)),
            (land(1, 1), Duration::from_millis(5)),
            (land(0, 1), Duration::from_millis(3)),
            (land(0, 2), Duration::from_millis(5)),
        ]
        .iter()
        .cloned()
        .collect();
        assert_eq!(&actual, &expected);
    }

    #[test]
    fn test_closest_targets() {
        let mut pathfinder = pathfinder();
        pathfinder.init_targets("targets".to_string());
        pathfinder.load_target("targets", &land(0, 2), true);
        pathfinder.load_target("targets", &land(1, 2), true);
        pathfinder.load_target("targets", &land(2, 2), true);
        let actual = pathfinder.closest_targets(&[land(1, 0)], "targets", 1);
        let expected = vec![ClosestTargetResult {
            position: land(1, 2),
            path: vec![land(1, 0), land(1, 1), land(1, 2)],
            duration: Duration::from_millis(6),
        }];
        assert_eq!(&actual, &expected);
    }

    #[test]
    fn test_manhattan_distance_single_target() {
        let pathfinder = pathfinder();
        let manhattan_distance = pathfinder.manhattan_distance(&[land(1, 2)]);
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&land(0, 0))),
            64 * 3
        );
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&water(0, 0))),
            64 * 3
        );
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&land(1, 2))),
            0
        );
    }

    #[test]
    fn test_manhattan_distance_multiple_targets() {
        let pathfinder = pathfinder();
        let manhattan_distance = pathfinder.manhattan_distance(&[land(0, 2), land(1, 2)]);
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&land(0, 0))),
            64 * 2
        );
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&water(0, 0))),
            64 * 2
        );
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&land(1, 2))),
            0
        );
    }

    #[test]
    fn test_in_bounds() {
        let pathfinder = pathfinder();
        assert!(pathfinder.in_bounds(&land(0, 0)));
        assert!(pathfinder.in_bounds(&land(1, 0)));
        assert!(pathfinder.in_bounds(&land(2, 0)));
        assert!(!pathfinder.in_bounds(&land(3, 0)));
        assert!(pathfinder.in_bounds(&land(0, 1)));
        assert!(pathfinder.in_bounds(&land(1, 1)));
        assert!(pathfinder.in_bounds(&land(2, 1)));
        assert!(!pathfinder.in_bounds(&land(3, 1)));
        assert!(pathfinder.in_bounds(&land(0, 2)));
        assert!(pathfinder.in_bounds(&land(1, 2)));
        assert!(pathfinder.in_bounds(&land(2, 2)));
        assert!(!pathfinder.in_bounds(&land(3, 2)));
        assert!(!pathfinder.in_bounds(&land(0, 3)));
        assert!(!pathfinder.in_bounds(&land(1, 3)));
        assert!(!pathfinder.in_bounds(&land(2, 3)));
        assert!(!pathfinder.in_bounds(&land(3, 3)));
    }
}
