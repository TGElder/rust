extern crate glutin;

use super::transform::Transform;
use commons::na;
use serde::{Deserialize, Serialize};

pub trait PhysicalPositionExt {
    fn to_gl_coord_2d(self, physical_size: glutin::dpi::PhysicalSize) -> GLCoord2D;
    fn to_buffer_coord(self, physical_size: glutin::dpi::PhysicalSize) -> BufferCoordinate;
    fn to_gl_coord_4d<T: ZFinder>(
        self,
        physical_size: glutin::dpi::PhysicalSize,
        z_finder: &T,
    ) -> GLCoord4D;
}

impl PhysicalPositionExt for glutin::dpi::PhysicalPosition {
    fn to_gl_coord_2d(self, physical_size: glutin::dpi::PhysicalSize) -> GLCoord2D {
        GLCoord2D {
            x: ((((self.x + 0.5) / physical_size.width) * 2.0) - 1.0) as f32,
            y: (1.0 - (((self.y + 0.5) / physical_size.height) * 2.0)) as f32,
        }
    }

    fn to_buffer_coord(self, physical_size: glutin::dpi::PhysicalSize) -> BufferCoordinate {
        let physical_position: (i32, i32) = self.into();
        BufferCoordinate {
            x: physical_position.0,
            y: (physical_size.height as i32) - physical_position.1,
        }
    }

    fn to_gl_coord_4d<T: ZFinder>(
        self,
        physical_size: glutin::dpi::PhysicalSize,
        z_finder: &T,
    ) -> GLCoord4D {
        let buffer_coord = self.to_buffer_coord(physical_size);
        let gl_coord_2d = self.to_gl_coord_2d(physical_size);
        GLCoord4D {
            x: gl_coord_2d.x,
            y: gl_coord_2d.y,
            z: z_finder.get_z_at(buffer_coord),
            w: 1.0,
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct BufferCoordinate {
    pub x: i32,
    pub y: i32,
}

impl BufferCoordinate {
    pub fn new(x: i32, y: i32) -> BufferCoordinate {
        BufferCoordinate { x, y }
    }
}

pub trait ZFinder {
    fn get_z_at(&self, buffer_coordinate: BufferCoordinate) -> f32;
}

#[derive(PartialEq, Debug)]
pub struct GLCoord2D {
    pub x: f32,
    pub y: f32,
}

impl GLCoord2D {
    pub fn new(x: f32, y: f32) -> GLCoord2D {
        GLCoord2D { x, y }
    }

    pub fn to_buffer_coord(&self, physical_size: glutin::dpi::PhysicalSize) -> BufferCoordinate {
        BufferCoordinate {
            x: ((((self.x + 1.0) / 2.0) * physical_size.width as f32) - 0.5).floor() as i32,
            y: ((((self.y + 1.0) / 2.0) * physical_size.height as f32) - 0.5).floor() as i32,
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct GLCoord3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl GLCoord3D {
    pub fn new(x: f32, y: f32, z: f32) -> GLCoord3D {
        GLCoord3D { x, y, z }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct GLCoord4D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl GLCoord4D {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> GLCoord4D {
        GLCoord4D { x, y, z, w }
    }

    pub fn to_world_coord(self, transformer: &Transform) -> WorldCoord {
        transformer.unproject(self)
    }

    pub fn round(&self) -> GLCoord4D {
        GLCoord4D {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
            w: self.w.round(),
        }
    }
}

impl Into<GLCoord4D> for na::Point4<f32> {
    fn into(self) -> GLCoord4D {
        GLCoord4D {
            x: self.x,
            y: self.y,
            z: self.z,
            w: self.w,
        }
    }
}

impl Into<na::Point4<f32>> for GLCoord4D {
    fn into(self) -> na::Point4<f32> {
        na::Point4::new(self.x, self.y, self.z, self.w)
    }
}

#[derive(PartialEq, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorldCoord {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl WorldCoord {
    pub fn new(x: f32, y: f32, z: f32) -> WorldCoord {
        WorldCoord { x, y, z }
    }

    pub fn to_gl_coord_4d(self, transformer: &Transform) -> GLCoord4D {
        transformer.project(self)
    }
}

impl Into<WorldCoord> for na::Point4<f32> {
    fn into(self) -> WorldCoord {
        WorldCoord {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

impl Into<na::Point4<f32>> for WorldCoord {
    fn into(self) -> na::Point4<f32> {
        na::Point4::new(self.x, self.y, self.z, 1.0)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use transform::Identity;

    #[test]
    fn physical_position_to_gl_2d_left_top() {
        let physical_size = glutin::dpi::PhysicalSize::new(100.0, 50.0);
        let physical_position = glutin::dpi::PhysicalPosition::new(0.0, 0.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GLCoord2D::new(-0.99, 0.98)
        );
    }

    #[test]
    fn physical_position_to_gl_2d_right_top() {
        let physical_size = glutin::dpi::PhysicalSize::new(100.0, 50.0);
        let physical_position = glutin::dpi::PhysicalPosition::new(100.0, 0.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GLCoord2D::new(1.01, 0.98)
        );
    }

    #[test]
    fn physical_position_to_gl_2d_left_bottom() {
        let physical_size = glutin::dpi::PhysicalSize::new(100.0, 50.0);
        let physical_position = glutin::dpi::PhysicalPosition::new(0.0, 50.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GLCoord2D::new(-0.99, -1.02)
        );
    }

    #[test]
    fn physical_position_to_gl_2d_right_bottom() {
        let physical_size = glutin::dpi::PhysicalSize::new(100.0, 50.0);
        let physical_position = glutin::dpi::PhysicalPosition::new(100.0, 50.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GLCoord2D::new(1.01, -1.02)
        );
    }

    #[test]
    fn physical_position_to_gl_2d_center() {
        let physical_size = glutin::dpi::PhysicalSize::new(100.0, 50.0);
        let physical_position = glutin::dpi::PhysicalPosition::new(50.0, 25.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GLCoord2D::new(0.01, -0.02)
        );
    }

    #[test]
    fn test_physical_position_to_gl_4d() {
        let physical_size = glutin::dpi::PhysicalSize::new(100.0, 50.0);
        let physical_position = glutin::dpi::PhysicalPosition::new(80.0, 10.0);

        struct MockZFinder {}
        impl ZFinder for MockZFinder {
            fn get_z_at(&self, buffer_coordinate: BufferCoordinate) -> f32 {
                assert_eq!(buffer_coordinate, BufferCoordinate::new(80, 40));
                2.22
            }
        }

        assert_eq!(
            physical_position.to_gl_coord_4d(physical_size, &MockZFinder {}),
            GLCoord4D::new(0.61, 0.58, 2.22, 1.0)
        );
    }

    #[test]
    fn test_gl_2d_to_buffer_coord() {
        let gl_coord_2 = GLCoord2D::new(-0.5, 0.5);
        let physical_size = glutin::dpi::PhysicalSize::new(256.0, 128.0);

        assert_eq!(
            gl_coord_2.to_buffer_coord(physical_size),
            BufferCoordinate::new(63, 95)
        );
    }

    #[test]
    fn test_gl_4d_to_world() {
        let transform = Transform::new(
            GLCoord3D::new(1.0, 2.0, 5.0),
            GLCoord2D::new(3.0, 4.0),
            Box::new(Identity::new()),
        );

        let gl_coord_4 = GLCoord4D::new(5.0, 6.0, 7.0, 8.0);
        let expected = transform.unproject(gl_coord_4);

        assert_eq!(gl_coord_4.to_world_coord(&transform), expected);
    }

    #[test]
    fn test_gl_4d_to_na_point4() {
        let gl_coord_4: GLCoord4D = GLCoord4D::new(1.0, 2.0, 3.0, 4.0);
        let point_4: na::Point4<f32> = gl_coord_4.into();
        assert_eq!(point_4, na::Point4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_na_point4_to_gl_4d() {
        let point_4 = na::Point4::new(1.0, 2.0, 3.0, 4.0);
        let gl_coord_4: GLCoord4D = point_4.into();
        assert_eq!(gl_coord_4, GLCoord4D::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_world_to_gl_4d() {
        let transform = Transform::new(
            GLCoord3D::new(1.0, 2.0, 5.0),
            GLCoord2D::new(3.0, 4.0),
            Box::new(Identity::new()),
        );

        let world_coord = WorldCoord::new(5.0, 6.0, 7.0);
        let expected = transform.project(world_coord);

        assert_eq!(world_coord.to_gl_coord_4d(&transform), expected);
    }

    #[test]
    fn test_world_to_na_point4() {
        let world_coord = WorldCoord::new(1.0, 2.0, 3.0);
        let point_4: na::Point4<f32> = world_coord.into();
        assert_eq!(point_4, na::Point4::new(1.0, 2.0, 3.0, 1.0));
    }

    #[test]
    fn test_na_point4_to_world() {
        let point_4 = na::Point4::new(1.0, 2.0, 3.0, 4.0);
        let world_coord: WorldCoord = point_4.into();
        assert_eq!(world_coord, WorldCoord::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn world_coord_round_trip() {
        let original = WorldCoord::new(0.1, 2.0, 30.0);
        let encoded: Vec<u8> = bincode::serialize(&original).unwrap();
        let reconstructed: WorldCoord = bincode::deserialize(&encoded[..]).unwrap();
        assert_eq!(original, reconstructed);
    }
}
