use crate::travel_duration::*;
use crate::world::*;
use commons::index2d::*;
use commons::*;
use network::Edge as NetworkEdge;
use network::Network;
use std::time::Duration;

pub struct Pathfinder<T>
where
    T: TravelDuration,
{
    index: Index2D,
    travel_duration: T,
    network: Network,
}

impl<T> Pathfinder<T>
where
    T: TravelDuration,
{
    pub fn new(world: &World, travel_duration: T) -> Pathfinder<T> {
        let mut out = Pathfinder {
            index: Index2D::new(world.width(), world.height()),
            travel_duration,
            network: Network::new(world.width() * world.height(), &[]),
        };
        out.compute_network(world);
        out
    }

    fn get_network_index(&self, position: &V2<usize>) -> Result<usize, PositionOutOfBounds> {
        self.index.get_index(position)
    }

    fn get_position_from_network_index(
        &self,
        network_index: usize,
    ) -> Result<V2<usize>, IndexOutOfBounds> {
        self.index.get_position(network_index)
    }

    fn get_network_edge(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<NetworkEdge> {
        self.travel_duration
            .get_cost(world, from, to)
            .and_then(|cost| {
                if let (Ok(from), Ok(to)) =
                    (self.get_network_index(from), self.get_network_index(to))
                {
                    Some(NetworkEdge::new(from, to, cost))
                } else {
                    None
                }
            })
    }

    fn compute_network_edges(&self, world: &World) -> Vec<NetworkEdge> {
        let mut edges = vec![];
        for y in 0..world.height() {
            for x in 0..world.width() {
                [
                    self.get_network_edge(&world, &v2(x, y), &v2(x + 1, y)),
                    self.get_network_edge(&world, &v2(x + 1, y), &v2(x, y)),
                    self.get_network_edge(&world, &v2(x, y), &v2(x, y + 1)),
                    self.get_network_edge(&world, &v2(x, y + 1), &v2(x, y)),
                ]
                .iter()
                .flatten()
                .for_each(|edge| edges.push(*edge))
            }
        }
        edges
    }

    pub fn compute_network(&mut self, world: &World) {
        self.network = Network::new(world.width() * world.height(), &[]);
        self.compute_network_edges(world)
            .iter()
            .for_each(|edge| self.network.add_edge(&edge));
    }

    pub fn manhattan_distance(&self, to: usize) -> impl Fn(usize) -> u32 {
        let to = self.get_position_from_network_index(to).unwrap();
        let to_x = to.x as i32;
        let to_y = to.y as i32;
        let index = self.index;
        let minimum_duration = self.travel_duration.min_duration();
        let minimum_cost = self
            .travel_duration
            .get_cost_from_duration_u8(minimum_duration) as u32;
        move |from| {
            let from = index.get_position(from).unwrap();
            ((from.x as i32 - to_x).abs() + (from.y as i32 - to_y).abs()) as u32 * minimum_cost
        }
    }

    pub fn find_path(&self, from: &V2<usize>, to: &V2<usize>) -> Option<Vec<V2<usize>>> {
        if let (Ok(from), Ok(to)) = (self.get_network_index(from), self.get_network_index(to)) {
            let path = self
                .network
                .find_path(from, to, None, &self.manhattan_distance(to));
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
        } else {
            None
        }
    }

    pub fn update_edge(&mut self, world: &World, from: &V2<usize>, to: &V2<usize>) {
        if let (Ok(from), Ok(to)) = (self.get_network_index(from), self.get_network_index(to)) {
            self.network.remove_edges(from, to);
        }
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

    pub fn positions_within(&self, positions: &[V2<usize>], duration: Duration) -> Vec<V2<usize>> {
        let indices: Vec<usize> = positions
            .iter()
            .flat_map(|position| self.get_network_index(position))
            .collect();
        let max_cost = self.travel_duration.get_cost_from_duration(duration);
        self.network
            .nodes_within(&indices, max_cost)
            .iter()
            .flat_map(|index| match self.get_position_from_network_index(*index) {
                Ok(position) => Some(position),
                _ => None,
            })
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
        Pathfinder::new(&world(), travel_duration())
    }

    #[test]
    fn test_get_network_index() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.get_network_index(&v2(0, 0)), Ok(0));
        assert_eq!(pathfinder.get_network_index(&v2(1, 0)), Ok(1));
        assert_eq!(pathfinder.get_network_index(&v2(2, 0)), Ok(2));
        assert_eq!(pathfinder.get_network_index(&v2(0, 1)), Ok(3));
        assert_eq!(pathfinder.get_network_index(&v2(1, 1)), Ok(4));
        assert_eq!(pathfinder.get_network_index(&v2(2, 1)), Ok(5));
        assert_eq!(pathfinder.get_network_index(&v2(0, 2)), Ok(6));
        assert_eq!(pathfinder.get_network_index(&v2(1, 2)), Ok(7));
        assert_eq!(pathfinder.get_network_index(&v2(2, 2)), Ok(8));
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
            pathfinder.find_path(&v2(2, 2), &v2(1, 0)),
            Some(vec![v2(2, 2), v2(2, 1), v2(1, 1), v2(1, 0),])
        );
    }

    #[test]
    fn test_find_path_impossible() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.find_path(&v2(2, 2), &v2(2, 0)), None);
    }

    #[test]
    fn test_find_path_length_0() {
        let pathfinder = pathfinder();
        assert_eq!(pathfinder.find_path(&v2(2, 2), &v2(2, 2)), None);
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
        let expected = vec![v2(0, 0), v2(1, 0), v2(1, 1), v2(0, 1), v2(0, 2)];
        assert!(same_elements(&actual, &expected));
    }
}
