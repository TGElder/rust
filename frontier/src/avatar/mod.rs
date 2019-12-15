mod artist;
mod path;
mod travel_duration;
mod travel_mode;

pub use artist::*;
use path::*;
pub use travel_duration::*;
pub use travel_mode::*;

use crate::travel_duration::*;
use crate::world::World;
use commons::{v2, V2};
use isometric::coords::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum Rotation {
    Left,
    Up,
    Right,
    Down,
}

impl Rotation {
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

    fn angle(self) -> f32 {
        match self {
            Rotation::Left => 4.0 * (PI / 4.0),
            Rotation::Up => 2.0 * (PI / 4.0),
            Rotation::Right => 0.0 * (PI / 4.0),
            Rotation::Down => 6.0 * (PI / 4.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum AvatarState {
    Stationary {
        position: V2<usize>,
        rotation: Rotation,
        thinking: bool,
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
                position: *path.final_position(),
                rotation: path.compute_final_rotation(),
                thinking: false,
            }),
            _ => None,
        }
    }

    pub fn rotate_clockwise(&self) -> Option<AvatarState> {
        if let AvatarState::Stationary {
            rotation,
            position,
            thinking,
        } = self
        {
            Some(AvatarState::Stationary {
                rotation: rotation.clockwise(),
                position: *position,
                thinking: *thinking,
            })
        } else {
            None
        }
    }

    pub fn rotate_anticlockwise(&self) -> Option<AvatarState> {
        if let AvatarState::Stationary {
            rotation,
            position,
            thinking,
        } = self
        {
            Some(AvatarState::Stationary {
                rotation: rotation.anticlockwise(),
                position: *position,
                thinking: *thinking,
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
    ) -> Option<AvatarState>
    where
        T: TravelDuration,
    {
        match self {
            AvatarState::Stationary { position: from, .. } => {
                if *from == positions[0] {
                    return Some(AvatarState::Walking(Path::new(
                        world,
                        positions,
                        travel_duration,
                        start_at,
                    )));
                }
            }
            AvatarState::Walking(path) => {
                if *path.final_point_arrival() != start_at {
                    return None;
                }
                if let Some(path) = path.extend(world, positions, travel_duration) {
                    return Some(AvatarState::Walking(path));
                }
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

    fn compute_world_coord_basic(&self, world: &World, instant: &u128) -> Option<WorldCoord> {
        match &self {
            AvatarState::Stationary { position, .. } => {
                Some(world.snap(WorldCoord::new(position.x as f32, position.y as f32, 0.0)))
            }
            AvatarState::Walking(path) => path.compute_world_coord(world, instant),
            _ => None,
        }
    }

    pub fn compute_world_coord(&self, world: &World, instant: &u128) -> Option<WorldCoord> {
        if let Some(WorldCoord { x, y, z }) = self.compute_world_coord_basic(world, instant) {
            Some(WorldCoord::new(x, y, z.max(world.sea_level())))
        } else {
            None
        }
    }
}

use std::time::Duration;

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
            rotation: Rotation::Up,
            thinking: false,
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
            rotation: Rotation::Up,
            thinking: false,
        };
        let new_state = state.walk_positions(
            &world,
            vec![v2(0, 0), v2(1, 0), v2(2, 0)],
            &travel_duration(),
            0,
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
            rotation: Rotation::Up,
            thinking: false,
        };
        let new_state = state.walk_positions(
            &world,
            vec![v2(0, 1), v2(1, 1), v2(2, 1)],
            &travel_duration(),
            1000,
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
        );
        assert_eq!(new_state, None,)
    }

    #[test]
    fn test_compute_world_coord_basic_stationary() {
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
            rotation: Rotation::Up,
            thinking: false,
        };
        assert_eq!(
            avatar.compute_world_coord(&world(), &0),
            Some(WorldCoord::new(1.0, 1.0, 1.0))
        );
    }

    #[test]
    fn test_compute_world_coord_basic_walking() {
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
            .compute_world_coord(&world, &(start + duration.as_micros() / 4))
            .unwrap();
        let expected = WorldCoord::new(1.0, 1.25, 1.25);
        assert!(((actual.x * 100.0).round() / 100.0).almost(expected.x));
        assert!(((actual.y * 100.0).round() / 100.0).almost(expected.y));
        assert!(((actual.z * 100.0).round() / 100.0).almost(expected.z));
    }

    #[test]
    fn test_compute_world_coord_under_sea_level() {
        let avatar = AvatarState::Stationary {
            position: v2(2, 1),
            rotation: Rotation::Up,
            thinking: false,
        };
        assert_eq!(
            avatar.compute_world_coord(&world(), &0),
            Some(WorldCoord::new(2.0, 1.0, 0.5))
        );
    }
}
