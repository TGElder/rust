mod artist;
mod path;
mod travel_duration;
mod travel_mode;

pub use artist::*;
use path::*;
pub use travel_duration::*;
pub use travel_mode::*;

use crate::pathfinder::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::{v2, V2};
use isometric::coords::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
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
            }),
            _ => None,
        }
    }

    pub fn rotate_clockwise(&self) -> Option<AvatarState> {
        if let AvatarState::Stationary { rotation, position } = self {
            Some(AvatarState::Stationary {
                rotation: rotation.clockwise(),
                position: *position,
            })
        } else {
            None
        }
    }

    pub fn rotate_anticlockwise(&self) -> Option<AvatarState> {
        if let AvatarState::Stationary { rotation, position } = self {
            Some(AvatarState::Stationary {
                rotation: rotation.anticlockwise(),
                position: *position,
            })
        } else {
            None
        }
    }

    pub fn forward_path(&self) -> Option<Vec<V2<usize>>> {
        if let AvatarState::Stationary {
            position: from,
            rotation,
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

    pub fn walk_forward<T>(
        &self,
        world: &World,
        pathfinder: &Pathfinder<T>,
        start_at: u128,
    ) -> Option<AvatarState>
    where
        T: TravelDuration,
    {
        if let Some(path) = self.forward_path() {
            if pathfinder
                .travel_duration()
                .get_duration(world, &path[0], &path[1])
                .is_some()
            {
                return Some(AvatarState::Walking(Path::new(
                    world,
                    path,
                    pathfinder.travel_duration(),
                    start_at,
                )));
            }
        }
        None
    }

    fn walk_path(
        &self,
        world: &World,
        positions: Vec<V2<usize>>,
        travel_duration: &TravelDuration,
        start_at: u128,
    ) -> AvatarState {
        AvatarState::Walking(Path::new(world, positions, travel_duration, start_at))
    }

    pub fn walk_to<T>(
        &self,
        world: &World,
        to: &V2<usize>,
        pathfinder: &Pathfinder<T>,
        start_at: u128,
    ) -> Option<AvatarState>
    where
        T: TravelDuration,
    {
        match self {
            AvatarState::Stationary { position: from, .. } => {
                if let Some(positions) = pathfinder.find_path(&from, to) {
                    return Some(self.walk_path(
                        &world,
                        positions,
                        pathfinder.travel_duration(),
                        start_at,
                    ));
                }
            }
            AvatarState::Walking(path) => {
                let mut path = path.stop(&start_at);
                if let Some(positions) = pathfinder.find_path(&path.final_position(), to) {
                    path.extend(world, positions[1..].to_vec(), pathfinder.travel_duration());
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

    fn pathfinder() -> Pathfinder<TestTravelDuration> {
        Pathfinder::new(&world(), travel_duration())
    }

    #[test]
    fn test_forward() {
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
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
    fn test_walk_forward() {
        fn test_walk(
            avatar: &AvatarState,
            world: &mut World,
            from: V2<usize>,
            to: V2<usize>,
            rotation: Rotation,
        ) -> AvatarState {
            let start_at = 0;
            let avatar = avatar
                .walk_forward(&world, &pathfinder(), start_at)
                .unwrap();
            let duration = travel_duration().get_duration(&world, &from, &to).unwrap();
            assert_eq!(
                &avatar,
                &AvatarState::Walking(Path::new(
                    &world,
                    vec![from, to],
                    &travel_duration(),
                    start_at
                ))
            );
            let avatar = avatar.evolve(&(start_at + duration.as_micros())).unwrap();
            assert_eq!(
                &avatar,
                &AvatarState::Stationary {
                    position: to,
                    rotation
                }
            );
            avatar
        }

        let mut world = world();
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
            rotation: Rotation::Up,
        };

        let avatar = test_walk(&avatar, &mut world, v2(1, 1), v2(1, 2), Rotation::Up);

        let avatar = avatar.rotate_clockwise().unwrap();
        let avatar = avatar.rotate_clockwise().unwrap();

        let avatar = test_walk(&avatar, &mut world, v2(1, 2), v2(1, 1), Rotation::Down);

        let avatar = avatar.rotate_clockwise().unwrap();

        let avatar = test_walk(&avatar, &mut world, v2(1, 1), v2(0, 1), Rotation::Left);

        let avatar = avatar.rotate_anticlockwise().unwrap();
        let avatar = avatar.rotate_anticlockwise().unwrap();

        test_walk(&avatar, &mut world, v2(0, 1), v2(1, 1), Rotation::Right);
    }

    #[test]
    fn test_walk_path() {
        let avatar = AvatarState::Absent;
        let world = world();
        let path = vec![v2(0, 0), v2(1, 0), v2(1, 1)];
        let start_at = 0;
        let avatar = avatar.walk_path(&world, path.clone(), &travel_duration(), start_at);
        assert_eq!(
            avatar,
            AvatarState::Walking(Path::new(&world, path, &travel_duration(), start_at))
        )
    }

    #[test]
    fn test_walk_to_while_walking() {
        let avatar = AvatarState::Absent;
        let world = world();
        let path = vec![v2(0, 0), v2(1, 0), v2(1, 1)];
        let start = 0;
        let avatar = avatar.walk_path(&world, path.clone(), &travel_duration(), start);
        let avatar = avatar
            .walk_to(&world, &v2(0, 0), &pathfinder(), start)
            .unwrap();
        assert_eq!(
            avatar,
            AvatarState::Walking(Path::new(
                &world,
                vec![v2(0, 0), v2(1, 0), v2(0, 0)],
                &travel_duration(),
                start
            ))
        )
    }

    #[test]
    fn test_cannot_walk_on_no_cost_edge() {
        let world = world();
        let avatar = AvatarState::Stationary {
            position: v2(2, 2),
            rotation: Rotation::Up,
        };
        assert_eq!(avatar.walk_forward(&world, &pathfinder(), 0), None);
    }

    #[test]
    fn test_compute_world_coord_basic_stationary() {
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
            rotation: Rotation::Up,
        };
        assert_eq!(
            avatar.compute_world_coord(&world(), &0),
            Some(WorldCoord::new(1.0, 1.0, 1.0))
        );
    }

    #[test]
    fn test_compute_world_coord_basic_walking() {
        let avatar = AvatarState::Stationary {
            position: v2(1, 1),
            rotation: Rotation::Up,
        };
        let world = world();
        let start = 0;
        let avatar = avatar.walk_forward(&world, &pathfinder(), start).unwrap();
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
        };
        assert_eq!(
            avatar.compute_world_coord(&world(), &0),
            Some(WorldCoord::new(2.0, 1.0, 0.5))
        );
    }

}
