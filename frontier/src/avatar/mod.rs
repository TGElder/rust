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
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Avatar {
    pub name: String,
    pub journey: Option<Journey>,
    pub load: AvatarLoad,
    pub color: Color,
    pub skin_color: Color,
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
