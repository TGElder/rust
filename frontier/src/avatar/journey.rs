use super::*;
use crate::bridge::Bridges;
use crate::travel_duration::*;
use crate::world::World;
use commons::edge::Edge;
use commons::grid::Grid;
use commons::V2;
use commons::V3;
use isometric::coords::*;
use std::ops::Add;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Journey {
    frames: Vec<Frame>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Frame {
    pub position: V2<usize>,
    pub elevation: f32,
    pub arrival: u128,
    pub vehicle: Vehicle,
    pub rotation: Rotation,
    pub load: AvatarLoad,
}

impl From<&Frame> for WorldCoord {
    fn from(frame: &Frame) -> Self {
        WorldCoord::new(
            frame.position.x as f32,
            frame.position.y as f32,
            frame.elevation,
        )
    }
}

impl From<&Frame> for V3<f32> {
    fn from(frame: &Frame) -> Self {
        V3::new(
            frame.position.x as f32,
            frame.position.y as f32,
            frame.elevation,
        )
    }
}

impl Journey {
    pub fn new(
        world: &World,
        positions: Vec<V2<usize>>,
        travel_duration: &dyn TravelDuration,
        vehicle_fn: &dyn VehicleFn,
        start_at: u128,
        bridges: &Bridges,
    ) -> Journey {
        Journey {
            frames: Journey::compute_frames(
                world,
                &positions,
                start_at,
                travel_duration,
                vehicle_fn,
                bridges,
            ),
        }
    }

    pub fn stationary(
        world: &World,
        position: V2<usize>,
        vehicle: Vehicle,
        rotation: Rotation,
    ) -> Journey {
        Journey {
            frames: vec![Frame {
                position,
                elevation: Self::get_elevation(world, &position),
                arrival: 0,
                vehicle,
                rotation,
                load: AvatarLoad::None,
            }],
        }
    }

    fn compute_frames(
        world: &World,
        positions: &[V2<usize>],
        start_at: u128,
        travel_duration: &dyn TravelDuration,
        vehicle_fn: &dyn VehicleFn,
        bridges: &Bridges,
    ) -> Vec<Frame> {
        let mut next_arrival_time = start_at;
        let mut out = Vec::with_capacity(positions.len());
        out.push(Frame {
            position: positions[0],
            elevation: Self::get_elevation(world, &positions[0]),
            arrival: next_arrival_time,
            vehicle: Self::vehicle(world, &positions[0], &positions[1], vehicle_fn, bridges),
            rotation: Self::rotation(&positions[0], &positions[1]),
            load: AvatarLoad::None,
        });
        for p in 0..positions.len() - 1 {
            let from = positions[p];
            let to = positions[p + 1];
            let duration = Self::travel_duration(world, &from, &to, travel_duration, bridges);
            next_arrival_time += duration.as_micros();
            out.push(Frame {
                position: to,
                elevation: Self::get_elevation(world, &to),
                arrival: next_arrival_time,
                vehicle: Self::vehicle(world, &from, &to, vehicle_fn, bridges),
                rotation: Self::rotation(&from, &to),
                load: AvatarLoad::None,
            });
        }
        out
    }

    fn vehicle(
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
        vehicle_fn: &dyn VehicleFn,
        bridges: &Bridges,
    ) -> Vehicle {
        vehicle_fn
            .vehicle_between(world, &from, &to)
            .or_else(|| {
                bridges
                    .get(&Edge::new(*from, *to))
                    .map(|bridge| bridge.vehicle)
            })
            .unwrap_or_else(|| {
                panic!(
                    "Tried to create avatar journey over edge without vehicle from {:?} to {:?}",
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
                "Tried to create avatar journey over invalid edge from {:?} to {:?}",
                from, to
            );
        }
    }

    fn travel_duration(
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
        travel_duration: &dyn TravelDuration,
        bridges: &Bridges,
    ) -> Duration {
        travel_duration
            .get_duration(world, &from, &to)
            .or_else(|| {
                bridges
                    .get(&Edge::new(*from, *to))
                    .map(|bridge| bridge.duration)
            })
            .unwrap_or_else(|| {
                panic!(
                    "Tried to create avatar journey over impassable edge from {:?} to {:?}",
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

    pub fn world_coord_at(&self, instant: &u128) -> WorldCoord {
        self.progress_at(instant).world_coord_at(instant)
    }

    pub fn progress_at(&self, instant: &u128) -> Progress {
        if *instant <= self.frames[0].arrival {
            return Progress::At(&self.frames[0]);
        }

        let final_frame = self.final_frame();
        if *instant >= final_frame.arrival {
            return Progress::At(final_frame);
        }

        let next_index = self.index_at(instant);

        Progress::Between {
            from: &self.frames[next_index - 1],
            to: &self.frames[next_index],
        }
    }

    fn index_at(&self, instant: &u128) -> usize {
        match self
            .frames
            .binary_search_by(|probe| probe.arrival.cmp(instant))
        {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn stop(self, instant: &u128) -> Journey {
        match self.progress_at(instant) {
            Progress::At(frame) => Journey {
                frames: vec![*frame],
            },
            Progress::Between { from, to } => Journey {
                frames: vec![*from, *to],
            },
        }
    }

    pub fn append(self, journey: Journey) -> Option<Journey> {
        if self.final_frame().position != journey.frames[0].position {
            return None;
        }

        Some(self + journey)
    }

    pub fn frames_between_times(&self, from_exclusive: &u128, to_inclusive: &u128) -> Vec<&Frame> {
        self.frames
            .iter()
            .filter(|frame| {
                let arrival = frame.arrival;
                arrival > *from_exclusive && arrival <= *to_inclusive
            })
            .collect()
    }

    pub fn with_pause_at_start(mut self, pause: u128) -> Journey {
        let first_frame = *unwrap_or!(self.frames.first(), return self);
        self.frames
            .iter_mut()
            .for_each(|Frame { arrival, .. }| *arrival += pause);
        self.frames.insert(0, first_frame);
        self
    }

    pub fn with_pause_at_end(mut self, pause: u128) -> Journey {
        let mut last_frame = *unwrap_or!(self.frames.last(), return self);
        last_frame.arrival += pause;
        self.frames.push(last_frame);
        self
    }

    pub fn then_rotate_clockwise(mut self) -> Journey {
        let mut last_frame = *unwrap_or!(self.frames.last(), return self);
        last_frame.rotation = last_frame.rotation.clockwise();
        self.frames.push(last_frame);
        self
    }

    pub fn then_rotate_anticlockwise(mut self) -> Journey {
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

    pub fn with_load(mut self, load: AvatarLoad) -> Journey {
        for mut frame in &mut self.frames {
            frame.load = load
        }
        self
    }
}

#[derive(Debug, PartialEq)]
pub enum Progress<'a> {
    At(&'a Frame),
    Between { from: &'a Frame, to: &'a Frame },
}

impl<'a> Progress<'a> {
    pub fn world_coord_at(&self, instant: &u128) -> WorldCoord {
        let (&from, &to) = match self {
            Progress::At(frame) => return (*frame).into(),
            Progress::Between { from, to } => (from, to),
        };

        let p_micros = instant - from.arrival;
        let edge_micros = to.arrival - from.arrival;
        let p = ((p_micros as f64) / (edge_micros as f64)) as f32;

        let from: V3<f32> = from.into();
        let to: V3<f32> = to.into();

        let interpolated = from + (to - from) * p;
        WorldCoord::new(interpolated.x, interpolated.y, interpolated.z)
    }

    fn to(&self) -> &Frame {
        match self {
            Progress::At(frame) => frame,
            Progress::Between { to, .. } => to,
        }
    }

    pub fn vehicle(&self) -> Vehicle {
        self.to().vehicle
    }

    pub fn rotation(&self) -> Rotation {
        self.to().rotation
    }

    pub fn load(&self) -> AvatarLoad {
        self.to().load
    }
}

impl Add for Journey {
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
        let actual = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            0,
            &hashmap! {},
        );
        let expected = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(0, 1),
                    elevation: 0.5,
                    arrival: 1_000,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 1.0,
                    arrival: 3_000,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 2),
                    elevation: 2.0,
                    arrival: 6_000,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(2, 2),
                    elevation: 3.0,
                    arrival: 10_000,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
            ],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_final_frame() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let journey = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            0,
            &hashmap! {},
        );
        assert_eq!(
            journey.final_frame(),
            &Frame {
                position: v2(2, 2),
                elevation: 3.0,
                arrival: 10_000,
                vehicle: Vehicle::None,
                rotation: Rotation::Right,
                load: AvatarLoad::None,
            }
        );
    }

    #[test]
    fn test_done() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let instant = 0;
        let journey = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            instant,
            &hashmap! {},
        );
        assert!(!journey.done(&instant));
        let done_at = instant + 10_000;
        assert!(journey.done(&done_at));
    }

    #[test]
    fn test_progress_at() {
        // Given
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let journey = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            start,
            &hashmap! {},
        );
        let at = start + 1_500;

        // When
        let progress = journey.progress_at(&at);

        // Then
        assert_eq!(
            progress,
            Progress::Between {
                from: &journey.frames[1],
                to: &journey.frames[2]
            }
        );
    }

    #[test]
    fn test_progress_at_before_start() {
        // Given
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 10;
        let journey = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            start,
            &hashmap! {},
        );

        // When
        let progress = journey.progress_at(&0);

        // Then
        assert_eq!(progress, Progress::At(&journey.frames[0]));
    }

    #[test]
    fn test_progress_at_after_end() {
        // Given
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;
        let journey = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            start,
            &hashmap! {},
        );

        // When
        let progress = journey.progress_at(&20_000);

        // Then
        assert_eq!(progress, Progress::At(&journey.frames[4]));
    }

    #[test]
    fn test_progress_enum_between() {
        // Given
        let from = Frame {
            position: v2(0, 1),
            elevation: 0.5,
            arrival: 1_000,
            vehicle: Vehicle::None,
            rotation: Rotation::Right,
            load: AvatarLoad::None,
        };
        let to = Frame {
            position: v2(1, 1),
            elevation: 1.0,
            arrival: 3_000,
            vehicle: Vehicle::Boat,
            rotation: Rotation::Up,
            load: AvatarLoad::Resource(Resource::Spice),
        };
        let progress = Progress::Between {
            from: &from,
            to: &to,
        };

        // Then
        let actual = progress.world_coord_at(&1_500);
        let expected = WorldCoord::new(0.25, 1.0, 0.625);
        assert!(actual.x.almost(&expected.x));
        assert!(actual.y.almost(&expected.y));
        assert!(actual.z.almost(&expected.z));

        assert_eq!(progress.vehicle(), Vehicle::Boat);
        assert_eq!(progress.rotation(), Rotation::Up);
        assert_eq!(progress.load(), AvatarLoad::Resource(Resource::Spice));
    }

    #[test]
    fn test_progress_enum_at() {
        // Given
        let at = Frame {
            position: v2(0, 1),
            elevation: 0.5,
            arrival: 1_000,
            vehicle: Vehicle::Boat,
            rotation: Rotation::Up,
            load: AvatarLoad::Resource(Resource::Spice),
        };
        let progress = Progress::At(&at);

        // Then
        let actual = progress.world_coord_at(&1_500);
        let expected = WorldCoord::new(0.0, 1.0, 0.5);
        assert!(actual.x.almost(&expected.x));
        assert!(actual.y.almost(&expected.y));
        assert!(actual.z.almost(&expected.z));

        assert_eq!(progress.vehicle(), Vehicle::Boat);
        assert_eq!(progress.rotation(), Rotation::Up);
        assert_eq!(progress.load(), AvatarLoad::Resource(Resource::Spice));
    }

    #[test]
    fn test_index_at() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 1;
        let journey = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            start,
            &hashmap! {},
        );
        assert_eq!(journey.index_at(&start), 1);
        let at = start + 1_500;
        assert_eq!(journey.index_at(&at), 2);
    }

    #[test]
    fn test_stop() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];
        let start = 0;

        let journey = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            start,
            &hashmap! {},
        );
        let frames = journey.frames.clone();

        assert_eq!(journey.clone().stop(&start).frames, vec![frames[0]]);
        let at = start + 1_500;
        assert_eq!(journey.stop(&at).frames, vec![frames[1], frames[2]]);
    }

    #[test]
    fn test_stop_after_finished() {
        let world = world();
        let positions = vec![v2(0, 0), v2(0, 1), v2(1, 1), v2(1, 2), v2(2, 2)];

        let journey = Journey::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            0,
            &hashmap! {},
        );
        let frames = journey.frames.clone();

        assert_eq!(journey.stop(&20000).frames, vec![frames[4]]);
    }

    #[test]
    fn test_stop_stationary() {
        let world = world();

        let journey = Journey::stationary(&world, v2(0, 0), Vehicle::None, Rotation::Up);
        let expected = journey.clone();

        assert_eq!(journey.stop(&1500), expected);
    }

    #[test]
    fn test_extend_compatible() {
        let a = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                    load: AvatarLoad::Resource(Resource::Crabs),
                },
            ],
        };
        let b = Journey {
            frames: vec![
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 2,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Left,
                    load: AvatarLoad::Resource(Resource::Crabs),
                },
                Frame {
                    position: v2(2, 2),
                    elevation: 3.0,
                    arrival: 3,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
            ],
        };
        let expected = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                    load: AvatarLoad::Resource(Resource::Crabs),
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 2,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Left,
                    load: AvatarLoad::Resource(Resource::Crabs),
                },
                Frame {
                    position: v2(2, 2),
                    elevation: 3.0,
                    arrival: 3,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
            ],
        };
        assert_eq!(a.append(b), Some(expected));
    }

    #[test]
    fn test_extend_incompatible() {
        let a = Journey {
            frames: vec![Frame {
                position: v2(0, 0),
                elevation: 1.0,
                arrival: 0,
                vehicle: Vehicle::None,
                rotation: Rotation::Up,
                load: AvatarLoad::None,
            }],
        };
        let b = Journey {
            frames: vec![Frame {
                position: v2(1, 1),
                elevation: 2.0,
                arrival: 1,
                vehicle: Vehicle::Boat,
                rotation: Rotation::Down,
                load: AvatarLoad::Resource(Resource::Crabs),
            }],
        };
        assert_eq!(a.append(b), None);
    }

    #[test]
    fn test_frames_between_times() {
        let journey = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 50,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 1.0,
                    arrival: 100,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 1.0,
                    arrival: 150,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
            ],
        };
        let actual = journey.frames_between_times(&75, &125);
        assert_eq!(actual, vec![&journey.frames[1]]);
    }

    #[test]
    fn test_frames_between_start_not_included() {
        let journey = Journey {
            frames: vec![Frame {
                position: v2(0, 0),
                elevation: 1.0,
                arrival: 50,
                vehicle: Vehicle::None,
                rotation: Rotation::Up,
                load: AvatarLoad::None,
            }],
        };
        let actual = journey.frames_between_times(&50, &125);
        assert!(actual.is_empty());
    }

    #[test]
    fn test_frames_between_end_is_included() {
        let journey = Journey {
            frames: vec![Frame {
                position: v2(0, 0),
                elevation: 1.0,
                arrival: 50,
                vehicle: Vehicle::None,
                rotation: Rotation::Up,
                load: AvatarLoad::None,
            }],
        };
        let actual = journey.frames_between_times(&0, &50);
        assert_eq!(actual, vec![&journey.frames[0]]);
    }

    #[test]
    fn test_frames_between_before_start_and_after_end() {
        let journey = Journey {
            frames: vec![Frame {
                position: v2(0, 0),
                elevation: 1.0,
                arrival: 50,
                vehicle: Vehicle::None,
                rotation: Rotation::Up,
                load: AvatarLoad::None,
            }],
        };
        let actual = journey.frames_between_times(&0, &100);
        assert_eq!(actual, vec![&journey.frames[0]]);
    }

    #[test]
    fn test_with_pause_at_start() {
        let journey = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                    load: AvatarLoad::Resource(Resource::Bananas),
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 2.0,
                    arrival: 10,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 20,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
            ],
        };

        let actual = journey.with_pause_at_start(1);

        let expected = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                    load: AvatarLoad::Resource(Resource::Bananas),
                },
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                    load: AvatarLoad::Resource(Resource::Bananas),
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 2.0,
                    arrival: 11,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 21,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
            ],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_start_empty() {
        let journey = Journey { frames: vec![] };
        let actual = journey.with_pause_at_start(1);
        let expected = Journey { frames: vec![] };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_end() {
        let journey = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 2.0,
                    arrival: 10,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 20,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                    load: AvatarLoad::Resource(Resource::Bananas),
                },
            ],
        };

        let actual = journey.with_pause_at_end(1);

        let expected = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 2.0,
                    arrival: 10,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 20,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                    load: AvatarLoad::Resource(Resource::Bananas),
                },
                Frame {
                    position: v2(2, 0),
                    elevation: 3.0,
                    arrival: 21,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Up,
                    load: AvatarLoad::Resource(Resource::Bananas),
                },
            ],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_with_pause_at_end_empty() {
        let journey = Journey { frames: vec![] };
        let actual = journey.with_pause_at_end(1);
        let expected = Journey { frames: vec![] };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_then_rotate_clockwise() {
        let journey = Journey::stationary(&world(), v2(0, 0), Vehicle::None, Rotation::Up);
        let journey = journey.then_rotate_clockwise();
        assert_eq!(
            journey,
            Journey {
                frames: vec![
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Up,
                        load: AvatarLoad::None,
                    },
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Right,
                        load: AvatarLoad::None,
                    }
                ]
            }
        );
    }

    #[test]
    fn test_then_rotate_anticlockwise() {
        let journey = Journey::stationary(&world(), v2(0, 0), Vehicle::None, Rotation::Up);
        let journey = journey.then_rotate_anticlockwise();
        assert_eq!(
            journey,
            Journey {
                frames: vec![
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Up,
                        load: AvatarLoad::None,
                    },
                    Frame {
                        position: v2(0, 0),
                        elevation: 1.0,
                        arrival: 0,
                        vehicle: Vehicle::None,
                        rotation: Rotation::Left,
                        load: AvatarLoad::None,
                    }
                ]
            }
        );
    }

    #[test]
    fn test_forward_path() {
        let journey = Journey::stationary(&world(), v2(1, 1), Vehicle::None, Rotation::Up);
        assert_eq!(journey.forward_path(), vec![v2(1, 1), v2(1, 2)]);

        let journey = Journey::stationary(&world(), v2(1, 1), Vehicle::None, Rotation::Down);
        assert_eq!(journey.forward_path(), vec![v2(1, 1), v2(1, 0)]);

        let journey = Journey::stationary(&world(), v2(1, 1), Vehicle::None, Rotation::Left);
        assert_eq!(journey.forward_path(), vec![v2(1, 1), v2(0, 1)]);

        let journey = Journey::stationary(&world(), v2(1, 1), Vehicle::None, Rotation::Right);
        assert_eq!(journey.forward_path(), vec![v2(1, 1), v2(2, 1)]);
    }

    #[test]
    fn test_with_load() {
        let a = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                    load: AvatarLoad::None,
                },
            ],
        };
        let expected = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::Resource(Resource::Deer),
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                    load: AvatarLoad::Resource(Resource::Deer),
                },
            ],
        };
        assert_eq!(a.with_load(AvatarLoad::Resource(Resource::Deer)), expected);
    }

    #[test]
    fn test_add() {
        let a = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                    load: AvatarLoad::Resource(Resource::Crabs),
                },
            ],
        };
        let b = Journey {
            frames: vec![
                Frame {
                    position: v2(2, 2),
                    elevation: 2.0,
                    arrival: 2,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Left,
                    load: AvatarLoad::Resource(Resource::Crabs),
                },
                Frame {
                    position: v2(3, 3),
                    elevation: 3.0,
                    arrival: 3,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
            ],
        };
        let expected = Journey {
            frames: vec![
                Frame {
                    position: v2(0, 0),
                    elevation: 1.0,
                    arrival: 0,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Up,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 1),
                    elevation: 2.0,
                    arrival: 1,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                    load: AvatarLoad::Resource(Resource::Crabs),
                },
                Frame {
                    position: v2(2, 2),
                    elevation: 2.0,
                    arrival: 2,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Left,
                    load: AvatarLoad::Resource(Resource::Crabs),
                },
                Frame {
                    position: v2(3, 3),
                    elevation: 3.0,
                    arrival: 3,
                    vehicle: Vehicle::None,
                    rotation: Rotation::Right,
                    load: AvatarLoad::None,
                },
            ],
        };
        assert_eq!(a + b, expected);
    }
}
