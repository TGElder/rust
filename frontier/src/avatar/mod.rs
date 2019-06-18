mod artist;
mod path;
mod travel_duration;
mod travel_mode;

pub use artist::*;
use path::*;
use travel_duration::*;
use travel_mode::*;

use crate::pathfinder::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::scale::*;
use commons::{v2, V2};
use isometric::coords::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::time::Instant;

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Rotation {
    Left,
    Up,
    Right,
    Down,
}

impl Rotation {
    fn clockwise(&self) -> Rotation {
        match self {
            Rotation::Left => Rotation::Up,
            Rotation::Up => Rotation::Right,
            Rotation::Right => Rotation::Down,
            Rotation::Down => Rotation::Left,
        }
    }

    fn anticlockwise(&self) -> Rotation {
        match self {
            Rotation::Left => Rotation::Down,
            Rotation::Up => Rotation::Left,
            Rotation::Right => Rotation::Up,
            Rotation::Down => Rotation::Right,
        }
    }

    fn angle(&self) -> f32 {
        match self {
            Rotation::Left => 4.0 * (PI / 4.0),
            Rotation::Up => 2.0 * (PI / 4.0),
            Rotation::Right => 0.0 * (PI / 4.0),
            Rotation::Down => 6.0 * (PI / 4.0),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AvatarState {
    Stationary {
        position: V2<usize>,
        rotation: Rotation,
    },
    Walking(Path),
}

pub struct Avatar {
    state: Option<AvatarState>,
    travel_mode_fn: TravelModeFn,
}

impl Avatar {
    pub fn new(min_navigable_river: f32) -> Avatar {
        Avatar {
            state: None,
            travel_mode_fn: TravelModeFn::new(min_navigable_river),
        }
    }

    pub fn travel_duration(&self) -> Box<TravelDuration> {
        let walk = GradientTravelDuration::boxed(Scale::new((-0.5, 0.5), (500.0, 1000.0)), false);
        let river = GradientTravelDuration::boxed(Scale::new((-0.1, 0.1), (250.0, 250.0)), false);
        let road = ConstantTravelDuration::boxed(Duration::from_millis(100));
        let sea = ConstantTravelDuration::boxed(Duration::from_millis(250));
        AvatarTravelDuration::boxed(self.travel_mode_fn.clone(), walk, road, river, sea)
    }

    pub fn rotation(&self, instant: &Instant) -> Option<Rotation> {
        match &self.state {
            Some(AvatarState::Stationary { rotation, .. }) => Some(*rotation),
            Some(AvatarState::Walking(path)) => path.compute_rotation(instant),
            None => None,
        }
    }

    pub fn state(&self) -> &Option<AvatarState> {
        &self.state
    }

    pub fn evolve(&mut self, instant: &Instant) {
        match self.state {
            Some(AvatarState::Walking(ref path)) if path.done(instant) => {
                self.state = Some(AvatarState::Stationary {
                    position: *path.final_position(),
                    rotation: path.compute_final_rotation(),
                })
            }
            _ => (),
        }
    }

    pub fn rotate_clockwise(&mut self) {
        if let Some(AvatarState::Stationary { position, rotation }) = self.state {
            self.state = Some(AvatarState::Stationary {
                position,
                rotation: rotation.clockwise(),
            })
        }
    }

    pub fn rotate_anticlockwise(&mut self) {
        if let Some(AvatarState::Stationary { position, rotation }) = self.state {
            self.state = Some(AvatarState::Stationary {
                position,
                rotation: rotation.anticlockwise(),
            })
        }
    }

    pub fn reposition(&mut self, position: V2<usize>, rotation: Rotation) {
        self.state = Some(AvatarState::Stationary { position, rotation });
    }

    pub fn forward_path(&self) -> Option<Vec<V2<usize>>> {
        if let Some(AvatarState::Stationary {
            position: from,
            rotation,
        }) = self.state
        {
            let to = v2(
                (from.x as f32 + rotation.angle().cos()).round() as usize,
                (from.y as f32 + rotation.angle().sin()).round() as usize,
            );
            return Some(vec![from, to]);
        }
        return None;
    }

    pub fn walk_forward(&mut self, world: &World, pathfinder: &Pathfinder, start_at: Instant) {
        if let Some(path) = self.forward_path() {
            if let Some(_) = pathfinder
                .travel_duration()
                .get_duration(world, &path[0], &path[1])
            {
                self.state = Some(AvatarState::Walking(Path::new(
                    world,
                    path,
                    &pathfinder.travel_duration(),
                    start_at,
                )));
            }
        }
    }

    fn walk_path(
        &mut self,
        world: &World,
        positions: Vec<V2<usize>>,
        travel_duration: &Box<TravelDuration>,
        start_at: Instant,
    ) {
        self.state = Some(AvatarState::Walking(Path::new(
            world,
            positions,
            travel_duration,
            start_at,
        )));
    }

    pub fn walk_to(
        &mut self,
        world: &World,
        to: &V2<usize>,
        pathfinder: &Pathfinder,
        start_at: Instant,
    ) {
        match self.state() {
            Some(AvatarState::Stationary { position: from, .. }) => {
                if let Some(positions) = pathfinder.find_path(&from, to) {
                    self.walk_path(&world, positions, pathfinder.travel_duration(), start_at);
                }
            }
            Some(AvatarState::Walking(path)) => {
                let mut path = path.stop(&start_at);
                if let Some(positions) = pathfinder.find_path(&path.final_position(), to) {
                    path.extend(world, positions[1..].to_vec(), pathfinder.travel_duration());
                    self.state = Some(AvatarState::Walking(path));
                }
            }
            None => (),
        }
    }

    pub fn stop(&mut self, stop_at: &Instant) {
        if let Some(AvatarState::Walking(path)) = self.state() {
            self.state = Some(AvatarState::Walking(path.stop(stop_at)));
        }
    }

    fn compute_world_coord_basic(&self, world: &World, instant: &Instant) -> Option<WorldCoord> {
        match &self.state {
            Some(AvatarState::Stationary { position, .. }) => {
                Some(world.snap(WorldCoord::new(position.x as f32, position.y as f32, 0.0)))
            }
            Some(AvatarState::Walking(path)) => path.compute_world_coord(world, instant),
            _ => None,
        }
    }

    pub fn compute_world_coord(&self, world: &World, instant: &Instant) -> Option<WorldCoord> {
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
    use commons::M;
    use std::time::Instant;

    fn travel_duration() -> Box<TravelDuration> {
        Box::new(TestTravelDuration {
            max: Duration::from_millis(4),
        })
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

    fn pathfinder() -> Pathfinder {
        Pathfinder::new(&world(), travel_duration())
    }

    fn avatar() -> Avatar {
        Avatar::new(0.0)
    }

    #[test]
    fn test_reposition() {
        let mut avatar = avatar();
        avatar.reposition(v2(1, 0), Rotation::Up);
        assert_eq!(
            avatar.state,
            Some(AvatarState::Stationary {
                position: v2(1, 0),
                rotation: Rotation::Up
            })
        );
    }

    #[test]
    fn test_forward() {
        let mut avatar = avatar();
        avatar.reposition(v2(1, 1), Rotation::Up);
        assert_eq!(avatar.forward_path(), Some(vec![v2(1, 1), v2(1, 2)]));
        avatar.rotate_clockwise();
        assert_eq!(avatar.forward_path(), Some(vec![v2(1, 1), v2(2, 1)]));
        avatar.rotate_clockwise();
        assert_eq!(avatar.forward_path(), Some(vec![v2(1, 1), v2(1, 0)]));
        avatar.rotate_clockwise();
        assert_eq!(avatar.forward_path(), Some(vec![v2(1, 1), v2(0, 1)]));
    }

    #[test]
    fn test_walk_forward() {
        fn test_walk(
            avatar: &mut Avatar,
            world: &mut World,
            from: V2<usize>,
            to: V2<usize>,
            rotation: Rotation,
        ) {
            let start_at = Instant::now();
            avatar.walk_forward(&world, &pathfinder(), start_at);
            let duration = travel_duration().get_duration(&world, &from, &to).unwrap();
            assert_eq!(
                avatar.state,
                Some(AvatarState::Walking(Path::new(
                    &world,
                    vec![from, to],
                    &travel_duration(),
                    start_at
                )))
            );
            avatar.evolve(&(start_at + duration));
            assert_eq!(
                avatar.state,
                Some(AvatarState::Stationary {
                    position: to,
                    rotation
                })
            );
        }

        let mut avatar = avatar();
        let mut world = world();
        avatar.reposition(v2(1, 1), Rotation::Up);

        test_walk(&mut avatar, &mut world, v2(1, 1), v2(1, 2), Rotation::Up);

        avatar.rotate_clockwise();
        avatar.rotate_clockwise();

        test_walk(&mut avatar, &mut world, v2(1, 2), v2(1, 1), Rotation::Down);

        avatar.rotate_clockwise();

        test_walk(&mut avatar, &mut world, v2(1, 1), v2(0, 1), Rotation::Left);

        avatar.rotate_anticlockwise();
        avatar.rotate_anticlockwise();

        test_walk(&mut avatar, &mut world, v2(0, 1), v2(1, 1), Rotation::Right);
    }

    #[test]
    fn test_walk_path() {
        let mut avatar = avatar();
        let world = world();
        let path = vec![v2(0, 0), v2(1, 0), v2(1, 1)];
        let start_at = Instant::now();
        avatar.walk_path(&world, path.clone(), &travel_duration(), start_at);
        assert_eq!(
            avatar.state(),
            &Some(AvatarState::Walking(Path::new(
                &world,
                path,
                &travel_duration(),
                start_at
            )))
        )
    }

    #[test]
    fn test_walk_to_while_walking() {
        let mut avatar = avatar();
        let world = world();
        let path = vec![v2(0, 0), v2(1, 0), v2(1, 1)];
        let start = Instant::now();
        avatar.walk_path(&world, path.clone(), &travel_duration(), start);
        avatar.walk_to(&world, &v2(0, 0), &pathfinder(), start);
        assert_eq!(
            avatar.state(),
            &Some(AvatarState::Walking(Path::new(
                &world,
                vec![v2(0, 0), v2(1, 0), v2(0, 0)],
                &travel_duration(),
                start
            )))
        )
    }

    #[test]
    fn test_cannot_walk_on_no_cost_edge() {
        let mut avatar = avatar();
        let world = world();
        avatar.reposition(v2(2, 2), Rotation::Up);
        avatar.walk_forward(&world, &pathfinder(), Instant::now());
        assert_eq!(
            avatar.state,
            Some(AvatarState::Stationary {
                position: v2(2, 2),
                rotation: Rotation::Up
            })
        );
    }

    #[test]
    fn test_compute_world_coord_basic_stationary() {
        let mut avatar = avatar();
        avatar.reposition(v2(1, 1), Rotation::Up);
        assert_eq!(
            avatar.compute_world_coord(&world(), &Instant::now()),
            Some(WorldCoord::new(1.0, 1.0, 1.0))
        );
    }

    #[test]
    fn test_compute_world_coord_basic_walking() {
        let mut avatar = avatar();
        let world = world();
        let start = Instant::now();
        avatar.reposition(v2(1, 1), Rotation::Up);
        avatar.walk_forward(&world, &pathfinder(), start);
        let duration = travel_duration()
            .get_duration(&world, &v2(1, 1), &v2(1, 2))
            .unwrap();
        let actual = avatar
            .compute_world_coord(&world, &(start + duration / 4))
            .unwrap();
        let expected = WorldCoord::new(1.0, 1.25, 1.25);
        assert_eq!((actual.x * 100.0).round() / 100.0, expected.x);
        assert_eq!((actual.y * 100.0).round() / 100.0, expected.y);
        assert_eq!((actual.z * 100.0).round() / 100.0, expected.z);
    }

    #[test]
    fn test_compute_world_coord_under_sea_level() {
        let mut avatar = avatar();
        avatar.reposition(v2(2, 1), Rotation::Up);
        assert_eq!(
            avatar.compute_world_coord(&world(), &Instant::now()),
            Some(WorldCoord::new(2.0, 1.0, 0.5))
        );
    }

}
