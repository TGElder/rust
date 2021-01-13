mod avatar_travel_mode_fn;
mod check_for_port;
mod path;
mod travel_duration;
mod travel_mode;
mod travel_mode_change;
mod travel_mode_fn;
mod vehicle;

pub use avatar_travel_mode_fn::*;
pub use check_for_port::*;
use path::*;
pub use travel_duration::*;
pub use travel_mode::*;
pub use travel_mode_change::*;
pub use travel_mode_fn::*;
pub use vehicle::*;

use crate::resource::Resource;
use crate::travel_duration::*;
use crate::world::World;
use commons::{v2, V2};
use isometric::coords::*;
use isometric::Color;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::f32::consts::PI;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Avatar {
    pub name: String,
    pub state: AvatarState,
    pub load: AvatarLoad,
    pub color: Color,
    pub skin_color: Color,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum AvatarState {
    Stationary {
        position: V2<usize>,
        elevation: f32,
        rotation: Rotation,
        vehicle: Vehicle,
    },
    Walking(Path),
    Absent,
}

impl AvatarState {
    pub fn rotation(&self, instant: &u128) -> Option<Rotation> {
        match &self {
            AvatarState::Stationary { rotation, .. } => Some(*rotation),
            AvatarState::Walking(path) => path.compute_rotation(instant),
            AvatarState::Absent => None,
        }
    }

    pub fn rotate_clockwise(&self) -> Option<AvatarState> {
        if let AvatarState::Stationary {
            position,
            elevation,
            rotation,
            vehicle,
        } = self
        {
            Some(AvatarState::Stationary {
                position: *position,
                elevation: *elevation,
                rotation: rotation.clockwise(),
                vehicle: *vehicle,
            })
        } else {
            None
        }
    }

    pub fn rotate_anticlockwise(&self) -> Option<AvatarState> {
        if let AvatarState::Stationary {
            position,
            elevation,
            rotation,
            vehicle,
        } = self
        {
            Some(AvatarState::Stationary {
                position: *position,
                elevation: *elevation,
                rotation: rotation.anticlockwise(),
                vehicle: *vehicle,
            })
        } else {
            None
        }
    }

    pub fn forward_path(&self) -> Option<Vec<V2<usize>>> {
        if let AvatarState::Stationary {
            position: from,
            rotation,
            ..
        } = self
        {
            let to = v2(
                (from.x as f32 + rotation.angle().cos()).round() as usize,
                (from.y as f32 + rotation.angle().sin()).round() as usize,
            );
            return Some(vec![*from, to]);
        }
        None
    }

    pub fn travel(&self, args: TravelArgs) -> Option<AvatarState> {
        match self {
            AvatarState::Stationary { position: from, .. } => {
                if *from != args.positions[0] {
                    return None;
                }
                Some(AvatarState::Walking(args.into()))
            }
            AvatarState::Walking(path) => {
                if path.final_frame().arrival != args.start_at {
                    return None;
                }
                let mut path = path.extend(
                    args.world,
                    args.positions,
                    args.travel_duration,
                    args.vehicle_fn,
                )?;
                if let Some(pause) = args.pause_at_end {
                    path = path.with_pause_at_end(pause.as_micros());
                }
                Some(AvatarState::Walking(path))
            }
            AvatarState::Absent => Some(AvatarState::Walking(args.into())),
        }
    }

    pub fn stop(&self, stop_at: &u128) -> Option<AvatarState> {
        if let AvatarState::Walking(path) = self {
            return Some(AvatarState::Walking(path.stop(stop_at)));
        }
        None
    }

    pub fn compute_world_coord(&self, instant: &u128) -> Option<WorldCoord> {
        match &self {
            AvatarState::Stationary {
                position,
                elevation,
                ..
            } => Some(WorldCoord::new(
                position.x as f32,
                position.y as f32,
                *elevation,
            )),
            AvatarState::Walking(path) => path.compute_world_coord(instant),
            _ => None,
        }
    }

    pub fn vehicle_at(&self, instant: &u128) -> Option<Vehicle> {
        match &self {
            AvatarState::Stationary { vehicle, .. } => Some(*vehicle),
            AvatarState::Walking(path) => path.vehicle_at(instant),
            _ => None,
        }
    }
}

pub struct TravelArgs<'a> {
    pub world: &'a World,
    pub positions: Vec<V2<usize>>,
    pub travel_duration: &'a dyn TravelDuration,
    pub vehicle_fn: &'a dyn VehicleFn,
    pub start_at: u128,
    pub pause_at_start: Option<Duration>,
    pub pause_at_end: Option<Duration>,
}

impl<'a> Into<Path> for TravelArgs<'a> {
    fn into(self) -> Path {
        let mut path = Path::new(
            self.world,
            self.positions,
            self.travel_duration,
            self.vehicle_fn,
            self.start_at,
        );
        if let Some(pause) = self.pause_at_start {
            path = path.with_pause_at_start(pause.as_micros());
        }
        if let Some(pause) = self.pause_at_end {
            path = path.with_pause_at_end(pause.as_micros());
        }
        path
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum Rotation {
    Left,
    Up,
    Right,
    Down,
}

impl Default for Rotation {
    fn default() -> Rotation {
        Rotation::Up
    }
}

impl Rotation {
    pub fn angle(self) -> f32 {
        match self {
            Rotation::Left => 4.0 * (PI / 4.0),
            Rotation::Up => 2.0 * (PI / 4.0),
            Rotation::Right => 0.0 * (PI / 4.0),
            Rotation::Down => 6.0 * (PI / 4.0),
        }
    }

    fn clockwise(self) -> Rotation {
        match self {
            Rotation::Left => Rotation::Up,
            Rotation::Up => Rotation::Right,
            Rotation::Right => Rotation::Down,
            Rotation::Down => Rotation::Left,
        }
    }

    fn anticlockwise(self) -> Rotation {
        match self {
            Rotation::Left => Rotation::Down,
            Rotation::Up => Rotation::Left,
            Rotation::Right => Rotation::Up,
            Rotation::Down => Rotation::Right,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum AvatarLoad {
    None,
    Resource(Resource),
}

#[allow(dead_code)]
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

    struct TestVehicleFn {}

    impl VehicleFn for TestVehicleFn {
        fn vehicle_between(&self, _: &World, _: &V2<usize>, _: &V2<usize>) -> Option<Vehicle> {
            Some(Vehicle::Boat)
        }
    }

    fn vehicle_fn() -> impl VehicleFn {
        TestVehicleFn {}
    }

    #[rustfmt::skip]
    fn world() -> World {
        World::new(
            M::from_vec(3, 3, vec![
                3.0, 2.0, 3.0,
                3.0, 1.0, 0.0,
                3.0, 2.0, 3.0,
            ]),
            0.5,
        )
    }

    #[test]
    fn test_forward_path() {
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
            elevation: 0.3,
            rotation: Rotation::Up,
            vehicle: Vehicle::None,
        };
        assert_eq!(avatar.forward_path(), Some(vec![v2(1, 1), v2(1, 2)]));
        let avatar = avatar.rotate_clockwise().unwrap();
        assert_eq!(avatar.forward_path(), Some(vec![v2(1, 1), v2(2, 1)]));
        let avatar = avatar.rotate_clockwise().unwrap();
        assert_eq!(avatar.forward_path(), Some(vec![v2(1, 1), v2(1, 0)]));
        let avatar = avatar.rotate_clockwise().unwrap();
        assert_eq!(avatar.forward_path(), Some(vec![v2(1, 1), v2(0, 1)]));
    }

    #[test]
    fn test_travel_stationary_compatible() {
        let world = world();
        let state = AvatarState::Stationary {
            position: v2(0, 0),
            elevation: 0.3,
            rotation: Rotation::Up,
            vehicle: Vehicle::None,
        };
        let new_state = state.travel(TravelArgs {
            world: &world,
            positions: vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            travel_duration: &travel_duration(),
            vehicle_fn: &vehicle_fn(),
            start_at: 0,
            pause_at_start: None,
            pause_at_end: None,
        });
        assert_eq!(
            new_state,
            Some(AvatarState::Walking(Path::new(
                &world,
                vec![v2(0, 0), v2(1, 0), v2(2, 0)],
                &travel_duration(),
                &vehicle_fn(),
                0
            )))
        )
    }

    #[test]
    fn test_travel_stationary_position_incompatible() {
        let world = world();
        let state = AvatarState::Stationary {
            position: v2(0, 0),
            elevation: 0.3,
            rotation: Rotation::Up,
            vehicle: Vehicle::None,
        };
        let new_state = state.travel(TravelArgs {
            world: &world,
            positions: vec![v2(0, 1), v2(1, 1), v2(2, 1)],
            travel_duration: &travel_duration(),
            vehicle_fn: &vehicle_fn(),
            start_at: 1000,
            pause_at_start: None,
            pause_at_end: None,
        });
        assert_eq!(new_state, None,)
    }

    #[test]
    fn test_travel_walking_compatible() {
        let world = world();
        let positions = vec![v2(0, 0), v2(1, 0)];
        let state = AvatarState::Walking(Path::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            0,
        ));
        let new_state = state.travel(TravelArgs {
            world: &world,
            positions: vec![v2(1, 0), v2(2, 0), v2(2, 1)],
            travel_duration: &travel_duration(),
            vehicle_fn: &vehicle_fn(),
            start_at: 1000,
            pause_at_start: None,
            pause_at_end: None,
        });
        assert_eq!(
            new_state,
            Some(AvatarState::Walking(Path::new(
                &world,
                vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
                &travel_duration(),
                &vehicle_fn(),
                0
            )))
        )
    }

    #[test]
    fn test_travel_walking_position_incompatible() {
        let world = world();
        let positions = vec![v2(0, 0), v2(1, 0)];
        let state = AvatarState::Walking(Path::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            0,
        ));
        let new_state = state.travel(TravelArgs {
            world: &world,
            positions: vec![v2(1, 1), v2(2, 1), v2(2, 2)],
            travel_duration: &travel_duration(),
            vehicle_fn: &vehicle_fn(),
            start_at: 1000,
            pause_at_start: None,
            pause_at_end: None,
        });
        assert_eq!(new_state, None,)
    }

    #[test]
    fn test_travel_walking_time_incompatible() {
        let world = world();
        let positions = vec![v2(0, 0), v2(1, 0)];
        let state = AvatarState::Walking(Path::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            0,
        ));
        let new_state = state.travel(TravelArgs {
            world: &world,
            positions: vec![v2(1, 0), v2(2, 0), v2(2, 1)],
            travel_duration: &travel_duration(),
            vehicle_fn: &vehicle_fn(),
            start_at: 2000,
            pause_at_start: None,
            pause_at_end: None,
        });
        assert_eq!(new_state, None,)
    }

    #[test]
    fn test_travel_stationary_with_pauses() {
        let world = world();
        let state = AvatarState::Stationary {
            position: v2(0, 0),
            elevation: 0.3,
            rotation: Rotation::Up,
            vehicle: Vehicle::None,
        };
        let new_state = state.travel(TravelArgs {
            world: &world,
            positions: vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            travel_duration: &travel_duration(),
            vehicle_fn: &vehicle_fn(),
            start_at: 0,
            pause_at_start: Some(Duration::from_secs(10)),
            pause_at_end: Some(Duration::from_secs(20)),
        });
        let expected_path = Path::new(
            &world,
            vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            &travel_duration(),
            &vehicle_fn(),
            0,
        )
        .with_pause_at_start(Duration::from_secs(10).as_micros())
        .with_pause_at_end(Duration::from_secs(20).as_micros());
        assert_eq!(new_state, Some(AvatarState::Walking(expected_path)),)
    }

    #[test]
    fn test_travel_walking_with_pauses() {
        let world = world();
        let positions = vec![v2(0, 0), v2(1, 0)];
        let state = AvatarState::Walking(Path::new(
            &world,
            positions,
            &travel_duration(),
            &vehicle_fn(),
            0,
        ));
        let new_state = state.travel(TravelArgs {
            world: &world,
            positions: vec![v2(1, 0), v2(2, 0), v2(2, 1)],
            travel_duration: &travel_duration(),
            vehicle_fn: &vehicle_fn(),
            start_at: 1000,
            pause_at_start: Some(Duration::from_secs(10)),
            pause_at_end: Some(Duration::from_secs(20)),
        });
        let expected_path = Path::new(
            &world,
            vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            &travel_duration(),
            &vehicle_fn(),
            0,
        )
        .with_pause_at_end(Duration::from_secs(20).as_micros());
        assert_eq!(new_state, Some(AvatarState::Walking(expected_path)),)
    }

    #[test]
    fn test_travel_absent() {
        let world = world();
        let state = AvatarState::Absent;
        let new_state = state.travel(TravelArgs {
            world: &world,
            positions: vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            travel_duration: &travel_duration(),
            vehicle_fn: &vehicle_fn(),
            start_at: 0,
            pause_at_start: None,
            pause_at_end: None,
        });
        assert_eq!(
            new_state,
            Some(AvatarState::Walking(Path::new(
                &world,
                vec![v2(0, 0), v2(1, 0), v2(2, 0)],
                &travel_duration(),
                &vehicle_fn(),
                0
            )))
        )
    }

    #[test]
    fn test_compute_world_coord_stationary() {
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
            elevation: 0.3,
            rotation: Rotation::Up,
            vehicle: Vehicle::None,
        };
        assert_eq!(
            avatar.compute_world_coord(&0),
            Some(WorldCoord::new(1.0, 1.0, 0.3))
        );
    }

    #[test]
    fn test_compute_world_coord_walking() {
        let world = world();
        let start = 0;
        let avatar = AvatarState::Walking(Path::new(
            &world,
            vec![v2(1, 1), v2(1, 2)],
            &travel_duration(),
            &vehicle_fn(),
            start,
        ));
        let duration = travel_duration()
            .get_duration(&world, &v2(1, 1), &v2(1, 2))
            .unwrap();
        let actual = avatar
            .compute_world_coord(&(start + duration.as_micros() / 4))
            .unwrap();
        let expected = WorldCoord::new(1.0, 1.25, 1.25);
        assert!(((actual.x * 100.0).round() / 100.0).almost(&expected.x));
        assert!(((actual.y * 100.0).round() / 100.0).almost(&expected.y));
        assert!(((actual.z * 100.0).round() / 100.0).almost(&expected.z));
    }

    #[test]
    fn test_vehicle_at_stationary() {
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
            elevation: 0.3,
            rotation: Rotation::Up,
            vehicle: Vehicle::None,
        };

        assert_eq!(avatar.vehicle_at(&123), Some(Vehicle::None));
    }

    #[test]
    fn test_vehicle_at_walking() {
        let world = world();
        let start = 0;
        let avatar = AvatarState::Walking(Path::new(
            &world,
            vec![v2(1, 1), v2(1, 2)],
            &travel_duration(),
            &vehicle_fn(),
            start,
        ));

        let duration = travel_duration()
            .get_duration(&world, &v2(1, 1), &v2(1, 2))
            .unwrap();
        let actual = avatar.vehicle_at(&(start + duration.as_micros() / 4));

        assert_eq!(actual, Some(Vehicle::Boat));
    }
}
