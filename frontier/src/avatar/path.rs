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
    frames: Vec<Frame>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Frame {
    pub position: V2<usize>,
    pub elevation: f32,
    pub arrival: u128,
    pub vehicle: Vehicle,
    pub rotation: Rotation,
}

impl Into<WorldCoord> for &Frame {
    fn into(self) -> WorldCoord {
        WorldCoord::new(
            self.position.x as f32,
            self.position.y as f32,
            self.elevation,
        )
    }
}

impl Path {
    pub fn new(
        world: &World,
        positions: Vec<V2<usize>>,
        travel_duration: &dyn TravelDuration,
        vehicle_fn: &dyn VehicleFn,
        start_at: u128,
    ) -> Path {
        Path {
            frames: Path::compute_frames(world, &positions, start_at, travel_duration, vehicle_fn),
        }
    }

    pub fn stationary(
        world: &World,
        position: V2<usize>,
        vehicle: Vehicle,
        rotation: Rotation,
    ) -> Path {
        Path {
            frames: vec![Frame {
                position,
                elevation: Self::get_elevation(world, &position),
                arrival: 0,
                vehicle,
                rotation,
            }],
        }
    }

    fn compute_frames(
        world: &World,
        positions: &[V2<usize>],
        start_at: u128,
        travel_duration: &dyn TravelDuration,
        vehicle_fn: &dyn VehicleFn,
    ) -> Vec<Frame> {
        let mut next_arrival_time = start_at;
        let mut out = Vec::with_capacity(positions.len());
        out.push(Frame {
            position: positions[0],
            elevation: Self::get_elevation(world, &positions[0]),
            arrival: next_arrival_time,
            vehicle: Self::vehicle(world, &positions[0], &positions[1], vehicle_fn),
            rotation: Self::rotation(&positions[0], &positions[1]),
        });
        for p in 0..positions.len() - 1 {
            let from = positions[p];
            let to = positions[p + 1];
            let duration = Self::travel_duration(world, &from, &to, travel_duration);
            next_arrival_time += duration.as_micros();
            out.push(Frame {
                position: to,
                elevation: Self::get_elevation(world, &to),
                arrival: next_arrival_time,
                vehicle: Self::vehicle(world, &from, &to, vehicle_fn),
                rotation: Self::rotation(&from, &to),
            });
        }
        out
    }

    fn vehicle(
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
        vehicle_fn: &dyn VehicleFn,
    ) -> Vehicle {
        vehicle_fn
            .vehicle_between(world, &from, &to)
            .unwrap_or_else(|| {
                panic!(
                    "Tried to create avatar path over edge without vehicle from {:?} to {:?}",
                    world.get_cell(from).unwrap(),
                    world.get_cell(to).unwrap()
                )
            })
    }

    fn rotation(from: &V2<usize>, to: &V2<usize>) -> Rotation {
        if to.x > from.x {
            Rotation::Right
        } else if from.x > to.x {
            Rotation::Left
        } else if to.y > from.y {
            Rotation::Up
        } else if from.y > to.y {
            Rotation::Down
        } else {
            panic!(
                "Tried to create avatar path over invalid edge from {:?} to {:?}",
                from, to
            );
        }
    }

    fn travel_duration(
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
        travel_duration: &dyn TravelDuration,
    ) -> Duration {
        travel_duration
            .get_duration(world, &from, &to)
            .unwrap_or_else(|| {
                panic!(
                    "Tried to create avatar path over impassable edge from {:?} to {:?}",
                    world.get_cell(from).unwrap(),
                    world.get_cell(to).unwrap()
                )
            })
    }

    fn get_elevation(world: &World, position: &V2<usize>) -> f32 {
        world
            .get_cell_unsafe(position)
            .elevation
            .max(world.sea_level())
    }

    pub fn final_frame(&self) -> &Frame {
        &self.frames[self.frames.len() - 1]
    }

    pub fn done(&self, instant: &u128) -> bool {
        *instant >= self.final_frame().arrival
    }

    fn compute_current_index(&self, instant: &u128) -> Option<usize> {
        for i in 0..self.frames.len() {
            if *instant < self.frames[i].arrival {
                return Some(i);
            }
        }
        None
    }

    pub fn stop(&self, instant: &u128) -> Path {
        self.compute_current_index(instant)
            .map(|i| Path {
                frames: vec![self.frames[i - 1], self.frames[i]],
            })
            .unwrap_or_else(|| Path {
                frames: vec![*self.final_frame()],
            })
    }

    pub fn compute_world_coord(&self, instant: &u128) -> WorldCoord {
        if self.done(instant) {
            return self.final_frame().into();
        }

        let instant = instant.max(&self.frames[0].arrival);

        let i = self.compute_current_index(instant).unwrap();

        let from = self.frames[i - 1];
        let to = self.frames[i];

        let p_micros = instant - from.arrival;
        let edge_micros = to.arrival - from.arrival;
        let p = ((p_micros as f64) / (edge_micros as f64)) as f32;

        let from = v3(
            from.position.x as f32,
            from.position.y as f32,
            from.elevation,
        );
        let to = v3(to.position.x as f32, to.position.y as f32, to.elevation);

        let interpolated = from + (to - from) * p;
        WorldCoord::new(interpolated.x, interpolated.y, interpolated.z)
    }

    pub fn vehicle_at(&self, instant: &u128) -> Vehicle {
        self.frame_at(instant).vehicle
    }

    pub fn rotation_at(&self, instant: &u128) -> Rotation {
        self.frame_at(instant).rotation
    }

    fn frame_at(&self, instant: &u128) -> &Frame {
        self.compute_current_index(instant)
            .map(|index| &self.frames[index])
            .unwrap_or_else(|| self.final_frame())
    }

    pub fn extend(
        self,
        world: &World,
        extension: Vec<V2<usize>>,
        travel_duration: &dyn TravelDuration,
        vehicle_fn: &dyn VehicleFn,
        start_at: u128,
    ) -> Option<Path> {
        if self.final_frame().position != extension[0] {
            return None;
        }

        let mut frames = self.frames;
        frames.append(&mut Path::compute_frames(
            world,
            &extension,
            start_at,
            travel_duration,
            vehicle_fn,
        ));

        Some(Path { frames })
    }

    fn compute_between_times<T>(
        &self,
        from_exclusive: &u128,
        to_inclusive: &u128,
        function: &dyn Fn(&Self, usize) -> T,
    ) -> Vec<T> {
        (0..self.frames.len())
            .filter(|i| {
                let arrival = self.frames[*i].arrival;
                arrival > *from_exclusive && arrival <= *to_inclusive
            })
            .map(|i| function(self, i))
            .collect()
    }

    pub fn edges_between_times(&self, from_exclusive: &u128, to_inclusive: &u128) -> Vec<Edge> {
        self.compute_between_times(from_exclusive, to_inclusive, &|s, i| {
            Edge::new(s.frames[i - 1].position, s.frames[i].position)
        })
    }

    pub fn with_pause_at_start(mut self, pause: u128) -> Path {
        let first_frame = *unwrap_or!(self.frames.first(), return self);
        self.frames
            .iter_mut()
            .for_each(|Frame { arrival, .. }| *arrival += pause);
        self.frames.insert(0, first_frame);
        self
    }

    pub fn with_pause_at_end(mut self, pause: u128) -> Path {
        let mut last_frame = *unwrap_or!(self.frames.last(), return self);
        last_frame.arrival += pause;
        self.frames.push(last_frame);
        self
    }

    pub fn then_rotate_clockwise(mut self) -> Path {
        let mut last_frame = *unwrap_or!(self.frames.last(), return self);
        last_frame.rotation = last_frame.rotation.clockwise();
        self.frames.push(last_frame);
        self
    }

    pub fn then_rotate_anticlockwise(mut self) -> Path {
        let mut last_frame = *unwrap_or!(self.frames.last(), return self);
        last_frame.rotation = last_frame.rotation.anticlockwise();
        self.frames.push(last_frame);
        self
    }

    pub fn forward_path(&self) -> Vec<V2<usize>> {
        let from = self.final_frame().position;
        let rotation = self.final_frame().rotation;
        let to = v2(
            (from.x as f32 + rotation.angle().cos()).round() as usize,
            (from.y as f32 + rotation.angle().sin()).round() as usize,
        );
        vec![from, to]
    }
}

impl Add for Path {
    type Output = Self;

    fn add(mut self, mut other: Self) -> Self {
        self.frames.append(&mut other.frames);
        Self {
            frames: self.frames,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::almost::Almost;
    use commons::*;

    struct TestTravelDuration {
        max: Duration,
    }

    impl TravelDuration for TestTravelDuration {
        fn get_duration(&self, _: &World, _: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
            if to.x <= 2 && to.y <= 2 {
                Some(Duration::from_millis((to.x + to.y) as u64))
            } else {
                None
            }
        }

        fn min_duration(&self) -> Duration {
            Duration::from_millis(0)
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

    struct TestVehicleFn {}

    impl VehicleFn for TestVehicleFn {
        fn vehicle_between(
            &self,
            world: &World,
            from: &V2<usize>,
            to: &V2<usize>,
        ) -> Option<Vehicle> {
            if world.get_cell_unsafe(from).elevation < world.sea_level()
                || world.get_cell_unsafe(to).elevation < world.sea_level()
            {
                Some(Vehicle::Boat)
            } else {
                Some(Vehicle::None)
            }
        }
    }

    fn vehicle_fn() -> impl VehicleFn {
        TestVehicleFn {}
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
    fn test_new() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let actual = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), 0);
        let expected = Path {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(0, 1),
                    elevation: 0.5,
                    arrival: 1_000,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 1.0,
                    arrival: 3_000,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Right,
                },
                Frame {
                    position: v2(1, 2),
                    elevation: 2.0,
                    arrival: 6_000,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(2, 2),
                    elevation: 3.0,
                    arrival: 10_000,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
            ],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_final_frame() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), 0);
        assert_eq!(
            path.final_frame(),
            &Frame {
                position: v2(2, 2),
                elevation: 3.0,
                arrival: 10_000,
                vehicle: Vehicle::None,
                rotation: Rotation::Right,
            }
        );
    }

    #[test]
    fn test_done() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let instant = 0;
        let path = Path::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            instant,
        );
        assert!(!path.done(&instant));
        let done_at = instant + 10_000;
        assert!(path.done(&done_at));
    }

    #[test]
    fn test_compute_current_index() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), start);
        assert_eq!(path.compute_current_index(&start), Some(1));
        let at = start + 1_500;
        assert_eq!(path.compute_current_index(&at), Some(2));
        let done_at = start + 10_000;
        assert_eq!(path.compute_current_index(&done_at), None);
    }

    #[test]
    fn test_compute_world_coord() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), start);
        let at = start + 1_500;
        let actual = path.compute_world_coord(&at);
        let expected = WorldCoord::new(0.25, 1.0, 0.625);
        assert!(actual.x.almost(&expected.x));
        assert!(actual.y.almost(&expected.y));
        assert!(actual.z.almost(&expected.z));
    }

    #[test]
    fn test_compute_world_coord_before_start() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 10;
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), start);

        let actual = path.compute_world_coord(&0);

        let expected = WorldCoord::new(0.0, 0.0, 1.0);
        assert!(actual.x.almost(&expected.x));
        assert!(actual.y.almost(&expected.y));
        assert!(actual.z.almost(&expected.z));
    }

    #[test]
    fn test_compute_world_coord_after_end() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), start);

        let actual = path.compute_world_coord(&20_000);

        let expected = WorldCoord::new(2.0, 2.0, 3.0);
        assert!(actual.x.almost(&expected.x));
        assert!(actual.y.almost(&expected.y));
        assert!(actual.z.almost(&expected.z));
    }

    #[test]
    fn test_vehicle_at() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), start);
        assert_eq!(path.vehicle_at(&0), Vehicle::Boat);
        assert_eq!(path.vehicle_at(&2_999), Vehicle::Boat);
        assert_eq!(path.vehicle_at(&3_000), Vehicle::None);
        assert_eq!(path.vehicle_at(&10_000), Vehicle::None);
    }

    #[test]
    fn test_rotation_at() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), start);
        assert_eq!(path.rotation_at(&0), Rotation::Up);
        assert_eq!(path.rotation_at(&2_999), Rotation::Right);
        assert_eq!(path.rotation_at(&3_000), Rotation::Up);
        assert_eq!(path.rotation_at(&10_000), Rotation::Right);
    }

    #[test]
    fn test_stop() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;

        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), start);
        let frames = path.frames.clone();

        assert_eq!(path.stop(&start).frames, vec![frames[0], frames[1]]);
        let at = start + 1_500;
        assert_eq!(path.stop(&at).frames, vec![frames[1], frames[2]]);
    }

    #[test]
    fn test_stop_after_finished() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];

        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), 0);
        let frames = path.frames.clone();

        assert_eq!(path.stop(&20000).frames, vec![frames[4]]);
    }

    #[test]
    fn test_stop_stationary() {
        let world = world();

        let path = Path::stationary(&world, v2(0, 0), Vehicle::None, Rotation::Up);
        let expected = path.clone();

        assert_eq!(path.stop(&1500), expected);
    }

    #[test]
    fn test_extend_compatible() {
        let world = world();
        let start = 0;
        let actual = Path::new(
            &world,
            vec![v2(0, 0), v2(0, 1)],
            &travel_duration(),
            &vehicle_fn(),
            start,
        );
        let actual = actual.extend(
            &world,
            vec![v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)],
            &travel_duration(),
            &vehicle_fn(),
            10_000,
        );
        assert_eq!(
            actual,
            Some(Path {
                frames: vec![
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::Boat,
                        rotation: Rotation::Up,
                    },
                    Frame {
                        position: v2(0, 1),
                        elevation: 0.5,
                        arrival: 1_000,
                        vehicle: Vehicle::Boat,
                        rotation: Rotation::Up,
                    },
                    Frame {
                        position: v2(0, 1),
                        elevation: 0.5,
                        arrival: 10_000,
                        vehicle: Vehicle::Boat,
                        rotation: Rotation::Right,
                    },
                    Frame {
                        position: v2(1, 1),
                        elevation: 1.0,
                        arrival: 12_000,
                        vehicle: Vehicle::Boat,
                        rotation: Rotation::Right,
                    },
                    Frame {
                        position: v2(1, 2),
                        elevation: 2.0,
                        arrival: 15_000,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Up,
                    },
                    Frame {
                        position: v2(2, 2),
                        elevation: 3.0,
                        arrival: 19_000,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Right,
                    },
                ],
            })
        );
    }

    #[test]
    fn test_extend_incompatible() {
        let world = world();
        let start = 0;
        let actual = Path::new(
            &world,
            vec![v2(0, 0), v2(0, 1)],
            &travel_duration(),
            &vehicle_fn(),
            start,
        );
        let actual = actual.extend(
            &world,
            vec![v2(1, 1), v2(1, 2), v2(2, 2)],
            &travel_duration(),
            &vehicle_fn(),
            10,
        );
        assert_eq!(actual, None);
    }

    #[test]
    fn test_edges_between_times() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), 0);
        let actual = path.edges_between_times(&1_500, &6_500);
        let expected = vec![Edge::new(v2(0, 1), v2(1, 1)), Edge::new(v2(1, 1), v2(1, 2))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_edges_between_times_start_not_included() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), 0);
        let actual = path.edges_between_times(&0, &1_500);
        let expected = vec![Edge::new(v2(0, 0), v2(0, 1))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_edges_between_times_end_is_included() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), 0);
        let actual = path.edges_between_times(&6_500, &10_000);
        let expected = vec![Edge::new(v2(1, 2), v2(2, 2))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_edges_between_times_before() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), 1_000);
        let actual = path.edges_between_times(&0, &500);
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_edges_between_times_after() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let path = Path::new(&world, positions, &travel_duration(), &vehicle_fn(), 0);
        let actual = path.edges_between_times(&10_000, &10_500);
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_start() {
        let path = Path {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 2.0,
                    arrival: 10,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 20,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
            ],
        };

        let actual = path.with_pause_at_start(1);

        let expected = Path {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 2.0,
                    arrival: 11,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 21,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
            ],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_start_empty() {
        let path = Path { frames: vec![] };
        let actual = path.with_pause_at_start(1);
        let expected = Path { frames: vec![] };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_end() {
        let path = Path {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 2.0,
                    arrival: 10,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 20,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                },
            ],
        };

        let actual = path.with_pause_at_end(1);

        let expected = Path {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 2.0,
                    arrival: 10,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 20,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 21,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                },
            ],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_end_empty() {
        let path = Path { frames: vec![] };
        let actual = path.with_pause_at_end(1);
        let expected = Path { frames: vec![] };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_then_rotate_clockwise() {
        let path = Path::stationary(&world(), v2(0, 0), Vehicle::None, Rotation::Up);
        let path = path.then_rotate_clockwise();
        assert_eq!(
            path,
            Path {
                frames: vec![
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Up,
                    },
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Right,
                    }
                ]
            }
        );
    }

    #[test]
    fn test_then_rotate_anticlockwise() {
        let path = Path::stationary(&world(), v2(0, 0), Vehicle::None, Rotation::Up);
        let path = path.then_rotate_anticlockwise();
        assert_eq!(
            path,
            Path {
                frames: vec![
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Up,
                    },
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Left,
                    }
                ]
            }
        );
    }

    #[test]
    fn test_forward_path() {
        let path = Path::stationary(&world(), v2(1, 1), Vehicle::None, Rotation::Up);
        assert_eq!(path.forward_path(), vec![v2(1, 1), v2(1, 2)]);

        let path = Path::stationary(&world(), v2(1, 1), Vehicle::None, Rotation::Down);
        assert_eq!(path.forward_path(), vec![v2(1, 1), v2(1, 0)]);

        let path = Path::stationary(&world(), v2(1, 1), Vehicle::None, Rotation::Left);
        assert_eq!(path.forward_path(), vec![v2(1, 1), v2(0, 1)]);

        let path = Path::stationary(&world(), v2(1, 1), Vehicle::None, Rotation::Right);
        assert_eq!(path.forward_path(), vec![v2(1, 1), v2(2, 1)]);
    }

    #[test]
    fn test_add() {
        let a = Path {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                },
            ],
        };
        let b = Path {
            frames: vec![
                Frame {
                    position: v2(2, 2),
                    elevation: 2.0,
                    arrival: 2,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Left,
                },
                Frame {
                    position: v2(3, 3),
                    elevation: 3.0,
                    arrival: 3,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
            ],
        };
        let expected = Path {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                },
                Frame {
                    position: v2(2, 2),
                    elevation: 2.0,
                    arrival: 2,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Left,
                },
                Frame {
                    position: v2(3, 3),
                    elevation: 3.0,
                    arrival: 3,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                },
            ],
        };
        assert_eq!(a + b, expected);
    }
}
