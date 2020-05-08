use crate::travel_duration::*;
use crate::world::*;
use commons::index2d::*;
use commons::*;
use network::ClosestTargetResult as NetworkClosestTargetResult;
use network::Edge as NetworkEdge;
use network::Network;
use std::collections::HashMap;
use std::time::Duration;

pub struct Pathfinder<T>
where
    T: TravelDuration,
{
    index: Index2D,
    travel_duration: T,
    network: Network,
}

#[derive(Debug, PartialEq)]
pub struct ClosestTargetResult {
    pub position: V2<usize>,
    pub path: Vec<V2<usize>>,
    pub duration: Duration,
}

impl<T> Pathfinder<T>
where
    T: TravelDuration,
{
    pub fn new(world: &World, travel_duration: T) -> Pathfinder<T> {
        Pathfinder {
            index: Index2D::new(world.width(), world.height()),
            travel_duration,
            network: Network::new(world.width() * world.height(), &[]),
        }
    }

    pub fn in_bounds(&self, position: &V2<usize>) -> bool {
        self.index.get_index(position).is_ok()
    }

    fn get_network_index(&self, position: &V2<usize>) -> usize {
        self.index.get_index(position).unwrap()
    }

    fn get_network_indices(&self, positions: &[V2<usize>]) -> Vec<usize> {
        positions
            .iter()
            .map(|position| self.get_network_index(position))
            .collect()
    }

    fn get_position_from_network_index(
        &self,
        network_index: usize,
    ) -> Result<V2<usize>, IndexOutOfBounds> {
        self.index.get_position(network_index)
    }

    fn get_positions_from_network_indices(&self, network_indices: &[usize]) -> Vec<V2<usize>> {
        network_indices
            .iter()
            .flat_map(|index| self.get_position_from_network_index(*index))
            .collect()
    }

    fn get_network_edge(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<NetworkEdge> {
        self.travel_duration.get_cost(world, from, to).map(|cost| {
            NetworkEdge::new(
                self.get_network_index(from),
                self.get_network_index(to),
                cost,
            )
        })
    }

    fn compute_network_edges(&self, world: &World) -> Vec<NetworkEdge> {
        let mut edges = vec![];
        for y in 0..world.height() {
            for x in 0..world.width() {
                [
                    (v2(x, y), v2(x + 1, y)),
                    (v2(x + 1, y), v2(x, y)),
                    (v2(x, y), v2(x, y + 1)),
                    (v2(x, y + 1), v2(x, y)),
                ]
                .iter()
                .filter(|edge| world.in_bounds(&edge.0) && world.in_bounds(&edge.1))
                .map(|edge| self.get_network_edge(&world, &edge.0, &edge.1))
                .flatten()
                .for_each(|edge| edges.push(edge));
            }
        }
        edges
    }

    pub fn reset_edges(&mut self, world: &World) {
        self.network.reset_edges();
        self.compute_network_edges(world)
            .iter()
            .for_each(|edge| self.network.add_edge(&edge));
    }

    pub fn manhattan_distance(&self, to: &[V2<usize>]) -> impl Fn(usize) -> u32 {
        let to: Vec<V2<i32>> = to.iter().map(|to| v2(to.x as i32, to.y as i32)).collect();
        let index = self.index;
        let minimum_duration = self.travel_duration.min_duration();
        let minimum_cost = self
            .travel_duration
            .get_cost_from_duration_u8(minimum_duration) as u32;
        move |from| {
            let from = index.get_position(from).unwrap();
            let from = v2(from.x as i32, from.y as i32);
            to.iter()
                .map(|to| ((from.x - to.x).abs() + (from.y - to.y).abs()) as u32 * minimum_cost)
                .min()
                .unwrap()
        }
    }

    pub fn find_path(&self, from: &[V2<usize>], to: &[V2<usize>]) -> Option<Vec<V2<usize>>> {
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
                let mut out = vec![];
                out.push(self.get_position_from_network_index(path[0].from).unwrap());
                for edge in path {
                    out.push(self.get_position_from_network_index(edge.to).unwrap());
                }
                Some(out)
            }
            None => None,
        }
    }

    pub fn update_edge(&mut self, world: &World, from: &V2<usize>, to: &V2<usize>) {
        self.network
            .remove_edges(self.get_network_index(from), self.get_network_index(to));
        if let Some(network_edge) = self.get_network_edge(&world, from, to) {
            self.network.add_edge(&network_edge);
        }
    }

    pub fn update_node(&mut self, world: &World, position: &V2<usize>) {
        for other in world.expand_position(position) {
            if other.x == position.x || other.y == position.y {
                self.update_edge(world, position, &other);
                self.update_edge(world, &other, position);
            }
        }
    }

    pub fn positions_within(
        &self,
        positions: &[V2<usize>],
        duration: Duration,
    ) -> HashMap<V2<usize>, Duration> {
        let indices = self.get_network_indices(positions);
        let max_cost = self.travel_duration.get_cost_from_duration(duration);
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

    pub fn load_target(&mut self, name: &str, position: &V2<usize>, target: bool) {
        self.network
            .load_target(name, self.get_network_index(position), target)
    }

    fn as_closest_target_result(&self, result: NetworkClosestTargetResult) -> ClosestTargetResult {
        ClosestTargetResult {
            position: self.get_position_from_network_index(result.node).unwrap(),
            path: self.get_positions_from_network_indices(&result.path),
            duration: self.travel_duration.get_duration_from_cost(result.cost),
        }
    }

    pub fn closest_targets(
        &self,
        positions: &[V2<usize>],
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
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::edge::*;
    use commons::M;
    use isometric::cell_traits::*;
    use std::time::Duration;

    struct TestTravelDuration {
        max: Duration,
    }

    impl TravelDuration for TestTravelDuration {
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
        let mut out = Pathfinder::new(world, travel_duration());
        out.reset_edges(world);
        out
    }

    #[test]
    fn test_in_bounds() {
        assert!(pathfinder().in_bounds(&v2(1, 1)));
    }

    #[test]
    fn test_out_of_bounds() {
        assert!(!pathfinder().in_bounds(&v2(3, 1)));
    }

    #[test]
    fn test_get_network_index() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.get_network_index(&v2(0, 0)), 0);
        assert_eq!(pathfinder.get_network_index(&v2(1, 0)), 1);
        assert_eq!(pathfinder.get_network_index(&v2(2, 0)), 2);
        assert_eq!(pathfinder.get_network_index(&v2(0, 1)), 3);
        assert_eq!(pathfinder.get_network_index(&v2(1, 1)), 4);
        assert_eq!(pathfinder.get_network_index(&v2(2, 1)), 5);
        assert_eq!(pathfinder.get_network_index(&v2(0, 2)), 6);
        assert_eq!(pathfinder.get_network_index(&v2(1, 2)), 7);
        assert_eq!(pathfinder.get_network_index(&v2(2, 2)), 8);
    }

    #[test]
    fn test_get_network_indices() {
        let pathfinder = pathfinder();
        let positions = [
            v2(0, 0),
            v2(1, 0),
            v2(2, 0),
            v2(0, 1),
            v2(1, 1),
            v2(2, 1),
            v2(0, 2),
            v2(1, 2),
            v2(2, 2),
        ];
        let actual = pathfinder.get_network_indices(&positions);
        assert_eq!(actual, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    #[should_panic]
    fn test_get_network_indices_out_of_bounds() {
        let pathfinder = pathfinder();
        let positions = [v2(3, 0)];
        pathfinder.get_network_indices(&positions);
    }

    #[test]
    fn test_get_network_position() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.get_position_from_network_index(0), Ok(v2(0, 0)));
        assert_eq!(pathfinder.get_position_from_network_index(1), Ok(v2(1, 0)));
        assert_eq!(pathfinder.get_position_from_network_index(2), Ok(v2(2, 0)));
        assert_eq!(pathfinder.get_position_from_network_index(3), Ok(v2(0, 1)));
        assert_eq!(pathfinder.get_position_from_network_index(4), Ok(v2(1, 1)));
        assert_eq!(pathfinder.get_position_from_network_index(5), Ok(v2(2, 1)));
        assert_eq!(pathfinder.get_position_from_network_index(6), Ok(v2(0, 2)));
        assert_eq!(pathfinder.get_position_from_network_index(7), Ok(v2(1, 2)));
        assert_eq!(pathfinder.get_position_from_network_index(8), Ok(v2(2, 2)));
    }

    #[test]
    fn test_get_positions_from_network_indices() {
        let pathfinder = pathfinder();
        let indices = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        let actual = pathfinder.get_positions_from_network_indices(&indices);
        assert_eq!(
            actual,
            vec![
                v2(0, 0),
                v2(1, 0),
                v2(2, 0),
                v2(0, 1),
                v2(1, 1),
                v2(2, 1),
                v2(0, 2),
                v2(1, 2),
                v2(2, 2)
            ]
        );
    }

    #[test]
    fn test_get_positions_from_network_indices_out_of_bounds() {
        let pathfinder = pathfinder();
        let indices = [9];
        let actual = pathfinder.get_positions_from_network_indices(&indices);
        assert_eq!(actual, vec![]);
    }

    #[test]
    fn test_get_network_edge() {
        let pathfinder = pathfinder();
        let world = world();
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(0, 0), &v2(1, 0)),
            Some(NetworkEdge::new(0, 1, 128))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 0), &v2(0, 0)),
            Some(NetworkEdge::new(1, 0, 255))
        );
    }

    #[test]
    fn test_get_network_edge_out_of_bounds() {
        let pathfinder = pathfinder();
        let world = world();
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(2, 0), &v2(3, 0)),
            None
        );
    }

    #[test]
    fn test_get_cost() {
        let world = world();
        let travel_duration = travel_duration();
        assert_eq!(
            travel_duration.get_cost(&world, &v2(0, 0), &v2(1, 0)),
            Some(128)
        );
        assert_eq!(
            travel_duration.get_cost(&world, &v2(1, 0), &v2(0, 0)),
            Some(255)
        );
        assert_eq!(travel_duration.get_cost(&world, &v2(1, 0), &v2(2, 0)), None);
    }

    #[test]
    #[should_panic]
    fn test_get_cost_duration_exceeds_max_duration() {
        let world = world();
        let travel_duration = TestTravelDuration {
            max: Duration::from_millis(1),
        };
        travel_duration.get_cost(&world, &v2(1, 0), &v2(0, 0));
    }

    #[test]
    fn test_compute_network_edges() {
        let edges = vec![
            NetworkEdge::new(0, 1, 128),
            NetworkEdge::new(1, 0, 255),
            NetworkEdge::new(0, 3, 191),
            NetworkEdge::new(3, 0, 255),
            NetworkEdge::new(2, 1, 128),
            NetworkEdge::new(1, 4, 191),
            NetworkEdge::new(4, 1, 128),
            NetworkEdge::new(2, 5, 128),
            NetworkEdge::new(3, 4, 191),
            NetworkEdge::new(4, 3, 191),
            NetworkEdge::new(3, 6, 128),
            NetworkEdge::new(6, 3, 191),
            NetworkEdge::new(4, 5, 128),
            NetworkEdge::new(5, 4, 191),
            NetworkEdge::new(4, 7, 191),
            NetworkEdge::new(7, 4, 191),
            NetworkEdge::new(5, 8, 255),
            NetworkEdge::new(8, 5, 128),
            NetworkEdge::new(6, 7, 191),
            NetworkEdge::new(7, 6, 128),
            NetworkEdge::new(7, 8, 255),
            NetworkEdge::new(8, 7, 191),
        ];

        assert_eq!(pathfinder().compute_network_edges(&world()), edges);
    }

    #[test]
    fn test_find_path() {
        let pathfinder = pathfinder();
        assert_eq!(
            pathfinder.find_path(&[v2(2, 2)], &[v2(1, 0)]),
            Some(vec![v2(2, 2), v2(2, 1), v2(1, 1), v2(1, 0),])
        );
    }

    #[test]
    fn test_find_path_impossible() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.find_path(&[v2(2, 2)], &[v2(2, 0)]), None);
    }

    #[test]
    fn test_find_path_length_0() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.find_path(&[v2(2, 2)], &[v2(2, 2)]), None);
    }

    #[test]
    fn test_find_path_multiple_from() {
        let pathfinder = pathfinder();
        assert_eq!(
            pathfinder.find_path(&[v2(0, 0), v2(1, 0)], &[v2(1, 2)]),
            Some(vec![v2(1, 0), v2(1, 1), v2(1, 2)])
        );
    }

    #[test]
    fn test_find_path_multiple_to() {
        let pathfinder = pathfinder();
        assert_eq!(
            pathfinder.find_path(&[v2(0, 0)], &[v2(2, 1), v2(0, 2)]),
            Some(vec![v2(0, 0), v2(0, 1), v2(0, 2)])
        );
    }

    #[test]
    fn test_update_edge() {
        let mut pathfinder = pathfinder();
        let mut world = world();
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 0), &v2(0, 0)),
            Some(NetworkEdge::new(1, 0, 255))
        );
        world.toggle_road(&Edge::new(v2(1, 0), v2(0, 0)));
        pathfinder.update_edge(&world, &v2(1, 0), &v2(0, 0));
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 0), &v2(0, 0)),
            Some(NetworkEdge::new(1, 0, 64))
        );
    }

    #[test]
    fn test_update_node() {
        let mut pathfinder = pathfinder();
        let mut world = world();
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 1), &v2(1, 0)),
            Some(NetworkEdge::new(4, 1, 128))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 0), &v2(1, 1)),
            Some(NetworkEdge::new(1, 4, 191))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 1), &v2(0, 1)),
            Some(NetworkEdge::new(4, 3, 191))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(0, 1), &v2(1, 1)),
            Some(NetworkEdge::new(3, 4, 191))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 1), &v2(2, 1)),
            Some(NetworkEdge::new(4, 5, 128))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(2, 1), &v2(1, 1)),
            Some(NetworkEdge::new(5, 4, 191))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 1), &v2(1, 2)),
            Some(NetworkEdge::new(4, 7, 191))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 2), &v2(1, 1)),
            Some(NetworkEdge::new(7, 4, 191))
        );
        world.toggle_road(&Edge::new(v2(1, 1), v2(1, 0)));
        world.toggle_road(&Edge::new(v2(1, 1), v2(0, 1)));
        world.toggle_road(&Edge::new(v2(1, 1), v2(2, 1)));
        world.toggle_road(&Edge::new(v2(1, 1), v2(1, 2)));
        pathfinder.update_node(&world, &v2(1, 1));
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 1), &v2(1, 0)),
            Some(NetworkEdge::new(4, 1, 64))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 0), &v2(1, 1)),
            Some(NetworkEdge::new(1, 4, 64))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 1), &v2(0, 1)),
            Some(NetworkEdge::new(4, 3, 64))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(0, 1), &v2(1, 1)),
            Some(NetworkEdge::new(3, 4, 64))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 1), &v2(2, 1)),
            Some(NetworkEdge::new(4, 5, 64))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(2, 1), &v2(1, 1)),
            Some(NetworkEdge::new(5, 4, 64))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 1), &v2(1, 2)),
            Some(NetworkEdge::new(4, 7, 64))
        );
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 2), &v2(1, 1)),
            Some(NetworkEdge::new(7, 4, 64))
        );
    }

    #[test]
    fn test_new_edge() {
        let mut pathfinder = pathfinder();
        let mut world = world();
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 0), &v2(2, 0)),
            None
        );
        world.toggle_road(&Edge::new(v2(1, 0), v2(2, 0)));
        pathfinder.update_edge(&world, &v2(1, 0), &v2(2, 0));
        assert_eq!(
            pathfinder.get_network_edge(&world, &v2(1, 0), &v2(2, 0)),
            Some(NetworkEdge::new(1, 2, 64))
        );
    }

    #[test]
    fn test_positions_within() {
        let pathfinder = pathfinder();
        let actual = pathfinder.positions_within(&[v2(0, 0)], Duration::from_millis(5));
        let expected = [
            (v2(0, 0), Duration::from_millis(0)),
            (v2(1, 0), Duration::from_millis(2)),
            (v2(1, 1), Duration::from_millis(5)),
            (v2(0, 1), Duration::from_millis(3)),
            (v2(0, 2), Duration::from_millis(5)),
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
        pathfinder.load_target("targets", &v2(0, 2), true);
        pathfinder.load_target("targets", &v2(1, 2), true);
        pathfinder.load_target("targets", &v2(2, 2), true);
        let actual = pathfinder.closest_targets(&[v2(1, 0)], "targets", 1);
        let expected = vec![ClosestTargetResult {
            position: v2(1, 2),
            path: vec![v2(1, 0), v2(1, 1), v2(1, 2)],
            duration: Duration::from_millis(6),
        }];
        assert_eq!(&actual, &expected);
    }

    #[test]
    fn test_manhattan_distance_single_target() {
        let pathfinder = pathfinder();
        let manhattan_distance = pathfinder.manhattan_distance(&[v2(1, 2)]);
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&v2(0, 0))),
            64 * 3
        );
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&v2(1, 2))),
            0
        );
    }

    #[test]
    fn test_manhattan_distance_multiple_targets() {
        let pathfinder = pathfinder();
        let manhattan_distance = pathfinder.manhattan_distance(&[v2(0, 2), v2(1, 2)]);
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&v2(0, 0))),
            64 * 2
        );
        assert_eq!(
            manhattan_distance(pathfinder.get_network_index(&v2(1, 2))),
            0
        );
    }
}
