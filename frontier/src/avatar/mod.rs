mod avatar_travel_mode_fn;
mod check_for_port;
mod journey;
mod travel_duration;
mod travel_mode;
mod travel_mode_change;
mod travel_mode_fn;
mod vehicle;

pub use avatar_travel_mode_fn::*;
pub use check_for_port::*;
use commons::edge::{DiagonalEdge, Edge};
pub use journey::*;
pub use travel_duration::*;
pub use travel_mode::*;
pub use travel_mode_change::*;
pub use travel_mode_fn::*;
pub use vehicle::*;

use crate::resource::Resource;
use crate::world::World;
use commons::{v2, V2};
use isometric::Color;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::f32::consts::PI;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Avatar {
    pub name: String,
    pub journey: Option<Journey>,
    pub color: Color,
    pub skin_color: Color,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum Rotation {
    Left = 0,
    Up = 1,
    Right = 2,
    Down = 3,
}

pub const ROTATIONS: [Rotation; 4] = [
    Rotation::Left,
    Rotation::Up,
    Rotation::Right,
    Rotation::Down,
];

impl Default for Rotation {
    fn default() -> Rotation {
        Rotation::Up
    }
}

impl Rotation {
    pub fn from_positions(from: &V2<usize>, to: &V2<usize>) -> Result<Rotation, DiagonalEdge> {
        Edge::new_safe(*from, *to)?;
        if to.x > from.x {
            Ok(Rotation::Right)
        } else if from.x > to.x {
            Ok(Rotation::Left)
        } else if to.y > from.y {
            Ok(Rotation::Up)
        } else if from.y > to.y {
            Ok(Rotation::Down)
        } else {
            Err(DiagonalEdge {
                from: *from,
                to: *to,
            })
        }
    }

    pub fn angle(self) -> f32 {
        match self {
            Rotation::Left => 4.0 * (PI / 4.0),
            Rotation::Up => 2.0 * (PI / 4.0),
            Rotation::Right => 0.0 * (PI / 4.0),
            Rotation::Down => 6.0 * (PI / 4.0),
        }
    }

    pub fn clockwise(self) -> Rotation {
        match self {
            Rotation::Left => Rotation::Up,
            Rotation::Up => Rotation::Right,
            Rotation::Right => Rotation::Down,
            Rotation::Down => Rotation::Left,
        }
    }

    pub fn anticlockwise(self) -> Rotation {
        match self {
            Rotation::Left => Rotation::Down,
            Rotation::Up => Rotation::Left,
            Rotation::Right => Rotation::Up,
            Rotation::Down => Rotation::Right,
        }
    }

    pub fn reverse(self) -> Rotation {
        match self {
            Rotation::Left => Rotation::Right,
            Rotation::Up => Rotation::Down,
            Rotation::Right => Rotation::Left,
            Rotation::Down => Rotation::Up,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum AvatarLoad {
    None,
    Resource(Resource),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_positions() {
        assert_eq!(
            Rotation::from_positions(&v2(1, 1), &v2(0, 1)),
            Ok(Rotation::Left)
        );
        assert_eq!(
            Rotation::from_positions(&v2(1, 1), &v2(2, 1)),
            Ok(Rotation::Right)
        );
        assert_eq!(
            Rotation::from_positions(&v2(1, 1), &v2(1, 0)),
            Ok(Rotation::Down)
        );
        assert_eq!(
            Rotation::from_positions(&v2(1, 1), &v2(1, 2)),
            Ok(Rotation::Up)
        );
        assert_eq!(
            Rotation::from_positions(&v2(1, 1), &v2(2, 2)),
            Err(DiagonalEdge {
                from: v2(1, 1),
                to: v2(2, 2)
            })
        );
    }
}
