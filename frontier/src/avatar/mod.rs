mod avatar_travel_mode_fn;
mod check_for_port;
mod path;
mod travel_duration;
mod travel_mode;
mod travel_mode_change;
mod travel_mode_fn;

pub use avatar_travel_mode_fn::*;
pub use check_for_port::*;
use path::*;
pub use travel_duration::*;
pub use travel_mode::*;
pub use travel_mode_change::*;
pub use travel_mode_fn::*;

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

    pub fn evolve(&self, instant: &u128) -> Option<AvatarState> {
        match self {
            AvatarState::Walking(ref path) if path.done(instant) => Some(AvatarState::Stationary {
                position: path.final_frame().position,
                elevation: path.final_frame().elevation,
                rotation: path.compute_final_rotation().unwrap_or_default(),
            }),
            _ => None,
        }
    }

    pub fn rotate_clockwise(&self) -> Option<AvatarState> {
        if let AvatarState::Stationary {
            position,
            elevation,
            rotation,
        } = self
        {
            Some(AvatarState::Stationary {
                position: *position,
                elevation: *elevation,
                rotation: rotation.clockwise(),
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
        } = self
        {
            Some(AvatarState::Stationary {
                position: *position,
                elevation: *elevation,
                rotation: rotation.anticlockwise(),
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

    pub fn walk_positions<T>(
        &self,
        world: &World,
        positions: Vec<V2<usize>>,
        travel_duration: &T,
        start_at: u128,
        pause_at_start: Option<Duration>,
        pause_at_end: Option<Duration>,
    ) -> Option<AvatarState>
    where
        T: TravelDuration,
    {
        match self {
            AvatarState::Stationary { position: from, .. } => {
                if *from != positions[0] {
                    return None;
                }
                let mut path = Path::new(world, positions, travel_duration, start_at);
                if let Some(pause) = pause_at_start {
                    path = path.with_pause_at_start(pause.as_micros());
                }
                if let Some(pause) = pause_at_end {
                    path = path.with_pause_at_end(pause.as_micros());
                }
                return Some(AvatarState::Walking(path));
            }
            AvatarState::Walking(path) => {
                if path.final_frame().arrival != start_at {
                    return None;
                }
                let mut path = path.extend(world, positions, travel_duration)?;
                if let Some(pause) = pause_at_end {
                    path = path.with_pause_at_end(pause.as_micros());
                }
                return Some(AvatarState::Walking(path));
            }
            _ => (),
        }
        None
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
    fn test_walk_positions_stationary_compatible() {
        let world = world();
        let state = AvatarState::Stationary {
            position: v2(0, 0),
            elevation: 0.3,
            rotation: Rotation::Up,
        };
        let new_state = state.walk_positions(
            &world,
            vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            &travel_duration(),
            0,
            None,
            None,
        );
        assert_eq!(
            new_state,
            Some(AvatarState::Walking(Path::new(
                &world,
                vec![v2(0, 0), v2(1, 0), v2(2, 0)],
                &travel_duration(),
                0
            )))
        )
    }

    #[test]
    fn test_walk_positions_stationary_position_incompatible() {
        let world = world();
        let state = AvatarState::Stationary {
            position: v2(0, 0),
            elevation: 0.3,
            rotation: Rotation::Up,
        };
        let new_state = state.walk_positions(
            &world,
            vec![v2(0, 1), v2(1, 1), v2(2, 1)],
            &travel_duration(),
            1000,
            None,
            None,
        );
        assert_eq!(new_state, None,)
    }

    #[test]
    fn test_walk_positions_walking_compatible() {
        let world = world();
        let positions = vec![v2(0, 0), v2(1, 0)];
        let state = AvatarState::Walking(Path::new(&world, positions, &travel_duration(), 0));
        let new_state = state.walk_positions(
            &world,
            vec![v2(1, 0), v2(2, 0), v2(2, 1)],
            &travel_duration(),
            1000,
            None,
            None,
        );
        assert_eq!(
            new_state,
            Some(AvatarState::Walking(Path::new(
                &world,
                vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
                &travel_duration(),
                0
            )))
        )
    }

    #[test]
    fn test_walk_positions_walking_position_incompatible() {
        let world = world();
        let positions = vec![v2(0, 0), v2(1, 0)];
        let state = AvatarState::Walking(Path::new(&world, positions, &travel_duration(), 0));
        let new_state = state.walk_positions(
            &world,
            vec![v2(1, 1), v2(2, 1), v2(2, 2)],
            &travel_duration(),
            1000,
            None,
            None,
        );
        assert_eq!(new_state, None,)
    }

    #[test]
    fn test_walk_positions_walking_time_incompatible() {
        let world = world();
        let positions = vec![v2(0, 0), v2(1, 0)];
        let state = AvatarState::Walking(Path::new(&world, positions, &travel_duration(), 0));
        let new_state = state.walk_positions(
            &world,
            vec![v2(1, 0), v2(2, 0), v2(2, 1)],
            &travel_duration(),
            2000,
            None,
            None,
        );
        assert_eq!(new_state, None,)
    }

    #[test]
    fn test_walk_positions_stationary_with_pauses() {
        let world = world();
        let state = AvatarState::Stationary {
            position: v2(0, 0),
            elevation: 0.3,
            rotation: Rotation::Up,
        };
        let new_state = state.walk_positions(
            &world,
            vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            &travel_duration(),
            0,
            Some(Duration::from_secs(10)),
            Some(Duration::from_secs(20)),
        );
        let expected_path = Path::new(
            &world,
            vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            &travel_duration(),
            0,
        )
        .with_pause_at_start(Duration::from_secs(10).as_micros())
        .with_pause_at_end(Duration::from_secs(20).as_micros());
        assert_eq!(new_state, Some(AvatarState::Walking(expected_path)),)
    }

    #[test]
    fn test_walk_positions_walking_with_pauses() {
        let world = world();
        let positions = vec![v2(0, 0), v2(1, 0)];
        let state = AvatarState::Walking(Path::new(&world, positions, &travel_duration(), 0));
        let new_state = state.walk_positions(
            &world,
            vec![v2(1, 0), v2(2, 0), v2(2, 1)],
            &travel_duration(),
            1000,
            Some(Duration::from_secs(10)),
            Some(Duration::from_secs(20)),
        );
        let expected_path = Path::new(
            &world,
            vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            &travel_duration(),
            0,
        )
        .with_pause_at_end(Duration::from_secs(20).as_micros());
        assert_eq!(new_state, Some(AvatarState::Walking(expected_path)),)
    }

    #[test]
    fn test_compute_world_coord_stationary() {
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
            elevation: 0.3,
            rotation: Rotation::Up,
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
}
