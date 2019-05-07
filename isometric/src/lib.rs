mod color;
pub mod coords;
mod engine;
pub mod event_handlers;
mod events;
mod font;
mod graphics;
pub mod terrain;
mod transform;
mod utils;

pub use color::Color;
pub use engine::*;
pub use events::*;
pub use font::*;
pub use graphics::drawing;
pub use graphics::texture::*;

pub use glutin::ElementState;
pub use glutin::MouseButton;
pub use glutin::VirtualKeyCode;

extern crate glutin;
pub extern crate image;
pub extern crate nalgebra as na;

use std::fmt::Debug;

pub type M<T> = na::DMatrix<T>;
pub type V2<T> = na::Vector2<T>;
pub type V3<T> = na::Vector3<T>;

pub fn v2<T: 'static + Copy + PartialEq + Debug>(x: T, y: T) -> na::Vector2<T> {
    na::Vector2::new(x, y)
}

pub fn v3<T: 'static + Copy + PartialEq + Debug>(x: T, y: T, z: T) -> na::Vector3<T> {
    na::Vector3::new(x, y, z)
}
