use super::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::{v2, V2};
use isometric::coords::*;
use std::ops::Add;
use std::time::Instant;

#[derive(Debug, PartialEq)]
pub struct Path {
    points: Vec<V2<usize>>,
    point_arrivals: Vec<Instant>,
}

impl Path {
    pub fn empty() -> Path {
        Path {
            points: vec![],
            point_arrivals: vec![],
        }
    }

    pub fn new(
        world: &World,
        points: Vec<V2<usize>>,
        travel_duration: &TravelDuration,
        start_at: Instant,
    ) -> Path {
        Path {
            point_arrivals: Path::compute_point_arrival_millis(
                world,
                &points,
                start_at,
                travel_duration,
            ),
            points,
        }
    }

    fn compute_point_arrival_millis(
        world: &World,
        points: &[V2<usize>],
        start_at: Instant,
        travel_duration: &TravelDuration,
    ) -> Vec<Instant> {
        let mut next_arrival_time = start_at;
        let mut out = Vec::with_capacity(points.len());
        out.push(next_arrival_time);
        for p in 0..points.len() - 1 {
            let from = points[p];
            let to = points[p + 1];
            if let Some(duration) = travel_duration.get_duration(world, &from, &to) {
                next_arrival_time += duration;
                out.push(next_arrival_time);
            } else {
                panic!(
                    "Tried to create avatar path over impassable edge from {:?} to {:?}",
                    from, to
                );
            }
        }
        out
    }

    pub fn final_position(&self) -> &V2<usize> {
        &self.points[self.points.len() - 1]
    }

    pub fn final_point_arrival(&self) -> &Instant {
        &self.point_arrivals[self.points.len() - 1]
    }

    pub fn done(&self, instant: &Instant) -> bool {
        instant >= self.final_point_arrival()
    }

    fn compute_current_index(&self, instant: &Instant) -> Option<usize> {
        for i in 0..self.points.len() {
            if *instant < self.point_arrivals[i] {
                return Some(i);
            }
        }
        None
    }

    pub fn stop(&self, instant: &Instant) -> Path {
        self.compute_current_index(instant)
            .map(|i| Path {
                points: vec![self.points[i - 1], self.points[i]],
                point_arrivals: vec![self.point_arrivals[i - 1], self.point_arrivals[i]],
            })
            .unwrap_or_else(Path::empty)
    }

    pub fn compute_world_coord(&self, world: &World, instant: &Instant) -> Option<WorldCoord> {
        self.compute_current_index(instant).map(|i| {
            let from = self.points[i - 1];
            let to = self.points[i];
            let from_time = self.point_arrivals[i - 1];
            let to_time = self.point_arrivals[i];
            let p_nanos = instant.duration_since(from_time).as_nanos();
            let edge_nanos = to_time.duration_since(from_time).as_nanos();
            let p = ((p_nanos as f64) / (edge_nanos as f64)) as f32;
            let from = v2(from.x as f32, from.y as f32);
            let to = v2(to.x as f32, to.y as f32);
            let x = from.x + (to.x - from.x) * p;
            let y = from.y + (to.y - from.y) * p;
            world.snap_to_edge(WorldCoord::new(x, y, 0.0))
        })
    }

    fn compute_rotation_at_index(&self, index: usize) -> Rotation {
        let from = self.points[index - 1];
        let to = self.points[index];
        if to.x > from.x {
            Rotation::Right
        } else if from.x > to.x {
            Rotation::Left
        } else if to.y > from.y {
            Rotation::Up
        } else if from.y > to.y {
            Rotation::Down
        } else {
            panic!("Avatar is walking between {:?} and {:?}. Cannot work out which direction avatar is facing.", from, to);
        }
    }

    pub fn compute_rotation(&self, instant: &Instant) -> Option<Rotation> {
        self.compute_current_index(instant)
            .map(|index| self.compute_rotation_at_index(index))
    }

    pub fn compute_final_rotation(&self) -> Rotation {
        self.compute_rotation_at_index(self.points.len() - 1)
    }

    pub fn extend(
        &mut self,
        world: &World,
        mut extension: Vec<V2<usize>>,
        travel_duration: &TravelDuration,
    ) {
        self.points.append(&mut extension);
        self.point_arrivals = Path::compute_point_arrival_millis(
            world,
            &self.points,
            self.point_arrivals[0],
            travel_duration,
        );
    }
}

impl Add for Path {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut points = vec![];
        let mut point_arrivals = vec![];
        points.append(&mut self.points.clone());
        points.append(&mut other.points.clone());
        point_arrivals.append(&mut self.point_arrivals.clone());
        point_arrivals.append(&mut other.point_arrivals.clone());
        Self {
            points,
            point_arrivals,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
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
        let instant = Instant::now();
        let actual =
            Path::compute_point_arrival_millis(&world, &points, instant, &travel_duration());
        let expected = vec![
            instant,
            instant + Duration::from_millis(1),
            instant + Duration::from_millis(3),
            instant + Duration::from_millis(6),
            instant + Duration::from_millis(10),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_final_position() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), Instant::now());
        assert_eq!(path.final_position(), &v2(2, 2));
    }

    #[test]
    fn test_final_point_arrival() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let instant = Instant::now();
        let path = Path::new(&world, points, &travel_duration(), instant);
        let expected = instant + Duration::from_millis(10);
        assert_eq!(path.final_point_arrival(), &expected);
    }

    #[test]
    fn test_done() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let instant = Instant::now();
        let path = Path::new(&world, points, &travel_duration(), instant);
        assert!(!path.done(&instant));
        let done_at = instant + Duration::from_millis(10);
        assert!(path.done(&done_at));
    }

    #[test]
    fn test_compute_current_index() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = Instant::now();
        let path = Path::new(&world, points, &travel_duration(), start);
        assert_eq!(path.compute_current_index(&start), Some(1));
        let at = start + Duration::from_micros(1500);
        assert_eq!(path.compute_current_index(&at), Some(2));
        let done_at = start + Duration::from_millis(10);
        assert_eq!(path.compute_current_index(&done_at), None);
    }

    #[test]
    fn test_compute_world_coord() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = Instant::now();
        let path = Path::new(&world, points, &travel_duration(), start);
        let at = start + Duration::from_micros(1500);
        let actual = path.compute_world_coord(&world, &at).unwrap();
        let expected = WorldCoord::new(0.25, 1.0, 0.25);
        assert!(((actual.x * 100.0).round() / 100.0).almost(expected.x));
        assert!(((actual.y * 100.0).round() / 100.0).almost(expected.y));
        assert!(((actual.z * 100.0).round() / 100.0).almost(expected.z));
    }

    #[test]
    fn test_compute_rotation_at_index() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(0, 1), v2(0, 0)];
        let path = Path::new(&world, points, &travel_duration(), Instant::now());
        assert_eq!(path.compute_rotation_at_index(1), Rotation::Up);
        assert_eq!(path.compute_rotation_at_index(2), Rotation::Right);
        assert_eq!(path.compute_rotation_at_index(3), Rotation::Left);
        assert_eq!(path.compute_rotation_at_index(4), Rotation::Down);
    }

    #[test]
    fn test_compute_rotation() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = Instant::now();
        let path = Path::new(&world, points, &travel_duration(), start);
        let at = start + Duration::from_micros(1500);
        let actual = path.compute_rotation(&at).unwrap();
        assert_eq!(actual, Rotation::Right);
    }

    #[test]
    fn test_final_rotation() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, points, &travel_duration(), Instant::now());
        assert_eq!(path.compute_final_rotation(), Rotation::Right);
    }

    #[test]
    fn test_stop() {
        let world = world();
        let points = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = Instant::now();
        let path = Path::new(&world, points, &travel_duration(), start);
        assert_eq!(path.stop(&start).points, vec![v2(0, 0), v2(0, 1)]);
        let at = start + Duration::from_micros(1500);
        assert_eq!(path.stop(&at).points, vec![v2(0, 1), v2(1, 1)]);
        let done_at = start + Duration::from_millis(10);
        assert_eq!(path.stop(&done_at).points, vec![]);
    }

    #[test]
    fn test_extend() {
        let world = world();
        let start = Instant::now();
        let mut actual = Path::new(&world, vec![v2(0, 0), v2(0, 1)], &travel_duration(), start);
        actual.extend(
            &world,
            vec![v2(1, 1), v2(1, 2), v2(2, 2)],
            &travel_duration(),
        );
        let expected = Path::new(
            &world,
            vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)],
            &travel_duration(),
            start,
        );
        assert_eq!(actual, expected);
    }

}
