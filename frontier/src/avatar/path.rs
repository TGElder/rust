use super::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::grid::Grid;
use commons::V2;
use commons::{edge::*, v3};
use isometric::coords::*;
use std::ops::Add;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Path {
    points: Vec<V2<usize>>,
    point_arrivals: Vec<u128>,
    elevations: Vec<f32>,
}

impl Path {
    pub fn empty() -> Path {
        Path {
            points: vec![],
            point_arrivals: vec![],
            elevations: vec![],
        }
    }

    pub fn new(
        world: &World,
        points: Vec<V2<usize>>,
        travel_duration: &dyn TravelDuration,
        start_at: u128,
    ) -> Path {
        Path {
            point_arrivals: Path::compute_point_arrival_millis(
                world,
                &points,
                start_at,
                travel_duration,
            ),
            elevations: Path::compute_elevations(world, &points),
            points,
        }
    }

    fn compute_point_arrival_millis(
        world: &World,
        points: &[V2<usize>],
        start_at: u128,
        travel_duration: &dyn TravelDuration,
    ) -> Vec<u128> {
        let mut next_arrival_time = start_at;
        let mut out = Vec::with_capacity(points.len());
        out.push(next_arrival_time);
        for p in 0..points.len() - 1 {
            let from = points[p];
            let to = points[p + 1];
            if let Some(duration) = travel_duration.get_duration(world, &from, &to) {
                next_arrival_time += duration.as_micros();
                out.push(next_arrival_time);
            } else {
                panic!(
                    "Tried to create avatar path over impassable edge from {:?} to {:?}",
                    world.get_cell(&from).unwrap(),
                    world.get_cell(&to).unwrap()
                );
            }
        }
        out
    }

    fn compute_elevations(world: &World, points: &[V2<usize>]) -> Vec<f32> {
        points
            .iter()
            .map(|point| {
                world
                    .get_cell_unsafe(point)
                    .elevation
                    .max(world.sea_level())
            })
            .collect()
    }

    pub fn final_position(&self) -> &V2<usize> {
        &self.points[self.points.len() - 1]
    }

    pub fn final_point_arrival(&self) -> &u128 {
        &self.point_arrivals[self.points.len() - 1]
    }

    pub fn final_elevation(&self) -> &f32 {
        &self.elevations[self.points.len() - 1]
    }

    pub fn done(&self, instant: &u128) -> bool {
        instant >= self.final_point_arrival()
    }

    fn compute_current_index(&self, instant: &u128) -> Option<usize> {
        for i in 0..self.points.len() {
            if *instant < self.point_arrivals[i] {
                return Some(i);
            }
        }
        None
    }

    pub fn stop(&self, instant: &u128) -> Path {
        self.compute_current_index(instant)
            .map(|i| Path {
                points: vec![self.points[i - 1], self.points[i]],
                point_arrivals: vec![self.point_arrivals[i - 1], self.point_arrivals[i]],
                elevations: vec![self.elevations[i - 1], self.elevations[i]],
            })
            .unwrap_or_else(Path::empty)
    }

    pub fn compute_world_coord(&self, instant: &u128) -> Option<WorldCoord> {
        let i = self.compute_current_index(instant)?;

        let from = self.points[i - 1];
        let to = self.points[i];
        let from_time = self.point_arrivals[i - 1];
        let to_time = self.point_arrivals[i];
        let from_z = self.elevations[i - 1];
        let to_z = self.elevations[i];

        let from = v3(from.x as f32, from.y as f32, from_z);
        let to = v3(to.x as f32, to.y as f32, to_z);

        let p_micros = instant - from_time;
        let edge_micros = to_time - from_time;
        let p = ((p_micros as f64) / (edge_micros as f64)) as f32;

        let interpolated = from + (to - from) * p;
        Some(WorldCoord::new(
            interpolated.x,
            interpolated.y,
            interpolated.z,
        ))
    }

    fn compute_rotation_at_index(&self, index: usize) -> Option<Rotation> {
        let from = self.points[index - 1];
        let to = self.points[index];
        if to.x > from.x {
            Some(Rotation::Right)
        } else if from.x > to.x {
            Some(Rotation::Left)
        } else if to.y > from.y {
            Some(Rotation::Up)
        } else if from.y > to.y {
            Some(Rotation::Down)
        } else {
            None
        }
    }

    pub fn compute_rotation(&self, instant: &u128) -> Option<Rotation> {
        self.compute_current_index(instant)
            .and_then(|index| self.compute_rotation_at_index(index))
    }

    pub fn compute_final_rotation(&self) -> Option<Rotation> {
        self.compute_rotation_at_index(self.points.len() - 1)
    }

    pub fn extend(
        &self,
        world: &World,
        extension: Vec<V2<usize>>,
        travel_duration: &dyn TravelDuration,
    ) -> Option<Path> {
        if *self.final_position() == extension[0] {
            let mut points: Vec<V2<usize>> = self.points.to_vec();
            points.append(&mut extension[1..].to_vec());
            let point_arrivals = Path::compute_point_arrival_millis(
                world,
                &points,
                self.point_arrivals[0],
                travel_duration,
            );
            let elevations = Path::compute_elevations(world, &points);
            Some(Path {
                points,
                point_arrivals,
                elevations,
            })
        } else {
            None
        }
    }

    fn compute_between_times<T>(
        &self,
        from_exclusive: &u128,
        to_inclusive: &u128,
        function: &dyn Fn(&Self, usize) -> T,
    ) -> Vec<T> {
        (0..self.points.len())
            .filter(|i| {
                let arrival = self.point_arrivals[*i];
                arrival > *from_exclusive && arrival <= *to_inclusive
            })
            .map(|i| function(self, i))
            .collect()
    }

    pub fn edges_between_times(&self, from_exclusive: &u128, to_inclusive: &u128) -> Vec<Edge> {
        self.compute_between_times(from_exclusive, to_inclusive, &|s, i| {
            Edge::new(s.points[i - 1], s.points[i])
        })
    }

    pub fn with_pause_at_start(mut self, pause: u128) -> Path {
        let first_point = *unwrap_or!(self.points.first(), return self);
        let first_arrival = *unwrap_or!(self.point_arrivals.first(), return self);
        let first_elevation = *unwrap_or!(self.elevations.first(), return self);
        self.point_arrivals
            .iter_mut()
            .for_each(|arrival| *arrival += pause);
        self.points.insert(0, first_point);
        self.point_arrivals.insert(0, first_arrival);
        self.elevations.insert(0, first_elevation);
        self
    }

    pub fn with_pause_at_end(mut self, pause: u128) -> Path {
        let last_point = *unwrap_or!(self.points.last(), return self);
        let last_arrival = *unwrap_or!(self.point_arrivals.last(), return self);
        let last_elevation = *unwrap_or!(self.elevations.last(), return self);
        self.points.push(last_point);
        self.point_arrivals.push(last_arrival + pause);
        self.elevations.push(last_elevation);
        self
    }
}

impl Add for Path {
    type Output = Self;

    fn add(mut self, mut other: Self) -> Self {
        self.points.append(&mut other.points);
        self.point_arrivals.append(&mut other.point_arrivals);
        self.elevations.append(&mut other.elevations);
        Self {
            points: self.points,
            point_arrivals: self.point_arrivals,
            elevations: self.elevations,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::almost::Almost;
    use commons::*;

    fn travel_duration() -> TestTravelDuration {
        TestTravelDuration {
            max: Duration::from_millis(4),
        }
    }

    #[rustfmt::skip]
    fn world() -> World {
        World::new(
            M::from_vec(3, 3, vec![
                1.0, 2.0, 3.0,
                0.0, 1.0, 0.0,
                3.0, 2.0, 3.0,
            ]),
            0.5,
        )
    }

    #[test]
    fn test_compute_point_arrival_millis() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let instant = 0;
        let actual =
            Path::compute_point_arrival_millis(&world, &points, instant, &travel_duration());
        let expected = vec![
            instant,
            instant + 1_000,
            instant + 3_000,
            instant + 6_000,
            instant + 10_000,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_compute_elevations() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let actual = Path::compute_elevations(&world, &points);
        let expected = vec![1.0, 0.5, 1.0, 2.0, 3.0];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_final_position() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), 0);
        assert_eq!(path.final_position(), &v2(2, 2));
    }

    #[test]
    fn test_final_point_arrival() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let instant = 0;
        let path = Path::new(&world, points, &travel_duration(), instant);
        let expected = instant + 10_000;
        assert_eq!(path.final_point_arrival(), &expected);
    }

    #[test]
    fn test_final_elevation() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), 0);
        assert!(path.final_elevation().almost(&3.0));
    }

    #[test]
    fn test_done() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let instant = 0;
        let path = Path::new(&world, points, &travel_duration(), instant);
        assert!(!path.done(&instant));
        let done_at = instant + 10_000;
        assert!(path.done(&done_at));
    }

    #[test]
    fn test_compute_current_index() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, points, &travel_duration(), start);
        assert_eq!(path.compute_current_index(&start), Some(1));
        let at = start + 1_500;
        assert_eq!(path.compute_current_index(&at), Some(2));
        let done_at = start + 10_000;
        assert_eq!(path.compute_current_index(&done_at), None);
    }

    #[test]
    fn test_compute_world_coord() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, points, &travel_duration(), start);
        let at = start + 1_500;
        let actual = path.compute_world_coord(&at).unwrap();
        let expected = WorldCoord::new(0.25, 1.0, 0.625);
        println!("{:?}", actual);
        println!("{:?}", expected);
        assert!(actual.x.almost(&expected.x));
        assert!(actual.y.almost(&expected.y));
        assert!(actual.z.almost(&expected.z));
    }

    #[test]
    fn test_compute_rotation_at_index() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(0, 1), v2(0, 0)];
        let path = Path::new(&world, points, &travel_duration(), 0);
        assert_eq!(path.compute_rotation_at_index(1), Some(Rotation::Up));
        assert_eq!(path.compute_rotation_at_index(2), Some(Rotation::Right));
        assert_eq!(path.compute_rotation_at_index(3), Some(Rotation::Left));
        assert_eq!(path.compute_rotation_at_index(4), Some(Rotation::Down));
    }

    #[test]
    fn test_compute_rotation() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, points, &travel_duration(), start);
        let at = start + 1_500;
        let actual = path.compute_rotation(&at).unwrap();
        assert_eq!(actual, Rotation::Right);
    }

    #[test]
    fn test_final_rotation() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), 0);
        assert_eq!(path.compute_final_rotation(), Some(Rotation::Right));
    }

    #[test]
    fn test_stop() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, points, &travel_duration(), start);
        assert_eq!(path.stop(&start).points, vec![v2(0, 0), v2(0, 1)]);
        let at = start + 1_500;
        assert_eq!(path.stop(&at).points, vec![v2(0, 1), v2(1, 1)]);
        let done_at = start + 10_000;
        assert!(path.stop(&done_at).points.is_empty());
    }

    #[test]
    fn test_extend_compatible() {
        let world = world();
        let start = 0;
        let actual = Path::new(&world, vec![v2(0, 0), v2(0, 1)], &travel_duration(), start);
        let actual = actual.extend(
            &world,
            vec![v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)],
            &travel_duration(),
        );
        let expected = Path::new(
            &world,
            vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)],
            &travel_duration(),
            start,
        );
        assert_eq!(actual, Some(expected));
    }

    #[test]
    fn test_extend_incompatible() {
        let world = world();
        let start = 0;
        let actual = Path::new(&world, vec![v2(0, 0), v2(0, 1)], &travel_duration(), start);
        let actual = actual.extend(
            &world,
            vec![v2(1, 1), v2(1, 2), v2(2, 2)],
            &travel_duration(),
        );
        assert_eq!(actual, None);
    }

    #[test]
    fn test_edges_between_times() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), 0);
        let actual = path.edges_between_times(&1_500, &6_500);
        let expected = vec![Edge::new(v2(0, 1), v2(1, 1)), Edge::new(v2(1, 1), v2(1, 2))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_edges_between_times_start_not_included() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), 0);
        let actual = path.edges_between_times(&0, &1_500);
        let expected = vec![Edge::new(v2(0, 0), v2(0, 1))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_edges_between_times_end_is_included() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), 0);
        let actual = path.edges_between_times(&6_500, &10_000);
        let expected = vec![Edge::new(v2(1, 2), v2(2, 2))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_edges_between_times_before() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), 1_000);
        let actual = path.edges_between_times(&0, &500);
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_edges_between_times_after() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), 0);
        let actual = path.edges_between_times(&10_000, &10_500);
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_start() {
        let path = Path {
            points: vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            point_arrivals: vec![0, 10, 20],
            elevations: vec![1.0, 2.0, 3.0],
        };
        let actual = path.with_pause_at_start(1);
        let expected = Path {
            points: vec![v2(0, 0), v2(0, 0), v2(1, 0), v2(2, 0)],
            point_arrivals: vec![0, 1, 11, 21],
            elevations: vec![1.0, 1.0, 2.0, 3.0],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_start_empty() {
        let path = Path {
            points: vec![],
            point_arrivals: vec![],
            elevations: vec![],
        };
        let actual = path.with_pause_at_start(1);
        let expected = Path {
            points: vec![],
            point_arrivals: vec![],
            elevations: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_end() {
        let path = Path {
            points: vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            point_arrivals: vec![0, 10, 20],
            elevations: vec![1.0, 2.0, 3.0],
        };
        let actual = path.with_pause_at_end(1);
        let expected = Path {
            points: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 0)],
            point_arrivals: vec![0, 10, 20, 21],
            elevations: vec![1.0, 2.0, 3.0, 3.0],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_end_empty() {
        let path = Path {
            points: vec![],
            point_arrivals: vec![],
            elevations: vec![],
        };
        let actual = path.with_pause_at_end(1);
        let expected = Path {
            points: vec![],
            point_arrivals: vec![],
            elevations: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add() {
        let a = Path {
            points: vec![v2(0, 0), v2(1, 1)],
            point_arrivals: vec![0, 1],
            elevations: vec![1.0, 2.0],
        };
        let b = Path {
            points: vec![v2(2, 2), v2(3, 3)],
            point_arrivals: vec![2, 3],
            elevations: vec![3.0, 4.0],
        };
        let expected = Path {
            points: vec![v2(0, 0), v2(1, 1), v2(2, 2), v2(3, 3)],
            point_arrivals: vec![0, 1, 2, 3],
            elevations: vec![1.0, 2.0, 3.0, 4.0],
        };
        assert_eq!(a + b, expected);
    }

    #[test]
    fn test_add_empty_lhs() {
        let a = Path::empty();
        let b = Path {
            points: vec![v2(2, 2), v2(3, 3)],
            point_arrivals: vec![2, 3],
            elevations: vec![3.0, 4.0],
        };
        assert_eq!(a + b.clone(), b);
    }

    #[test]
    fn test_add_empty_rhs() {
        let a = Path {
            points: vec![v2(0, 0), v2(1, 1)],
            point_arrivals: vec![0, 1],
            elevations: vec![1.0, 2.0],
        };
        let b = Path::empty();
        assert_eq!(a.clone() + b, a);
    }

    #[test]
    fn test_add_both_empty() {
        let a = Path::empty();
        let b = Path::empty();
        assert_eq!(a + b, Path::empty());
    }
}
