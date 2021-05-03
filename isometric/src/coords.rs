extern crate glutin;

use super::transform::Transform;
use commons::na;
use commons::{v2, V2};
use serde::{Deserialize, Serialize};

pub trait PhysicalPositionExt {
    fn to_gl_coord_2d(&self, physical_size: glutin::dpi::PhysicalSize<u32>) -> GlCoord2D;
    fn to_buffer_coord(&self, physical_size: glutin::dpi::PhysicalSize<u32>) -> BufferCoordinate;
    fn to_gl_coord_4d<T: ZFinder>(
        &self,
        physical_size: glutin::dpi::PhysicalSize<u32>,
        z_finder: &T,
    ) -> GlCoord4D;
}

impl PhysicalPositionExt for glutin::dpi::PhysicalPosition<f64> {
    fn to_gl_coord_2d(&self, physical_size: glutin::dpi::PhysicalSize<u32>) -> GlCoord2D {
        GlCoord2D {
            x: ((((self.x + 0.5) / physical_size.width as f64) * 2.0) - 1.0) as f32,
            y: (1.0 - (((self.y + 0.5) / physical_size.height as f64) * 2.0)) as f32,
        }
    }

    fn to_buffer_coord(&self, physical_size: glutin::dpi::PhysicalSize<u32>) -> BufferCoordinate {
        let physical_position: (i32, i32) = (*self).into();
        BufferCoordinate {
            x: physical_position.0,
            y: (physical_size.height as i32) - physical_position.1,
        }
    }

    fn to_gl_coord_4d<T: ZFinder>(
        &self,
        physical_size: glutin::dpi::PhysicalSize<u32>,
        z_finder: &T,
    ) -> GlCoord4D {
        let buffer_coord = self.to_buffer_coord(physical_size);
        let gl_coord_2d = self.to_gl_coord_2d(physical_size);
        GlCoord4D {
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GlCoord2D {
    pub x: f32,
    pub y: f32,
}

impl GlCoord2D {
    pub fn new(x: f32, y: f32) -> GlCoord2D {
        GlCoord2D { x, y }
    }

    pub fn to_buffer_coord(
        self,
        physical_size: &glutin::dpi::PhysicalSize<u32>,
    ) -> BufferCoordinate {
        BufferCoordinate {
            x: ((((self.x + 1.0) / 2.0) * physical_size.width as f32) - 0.5).floor() as i32,
            y: ((((self.y + 1.0) / 2.0) * physical_size.height as f32) - 0.5).floor() as i32,
        }
    }

    pub fn to_gl_coord_3d(
        self,
        physical_size: &glutin::dpi::PhysicalSize<u32>,
        z_finder: &dyn ZFinder,
    ) -> GlCoord3D {
        let buffer_coord = self.to_buffer_coord(physical_size);
        GlCoord3D::new(self.x, self.y, z_finder.get_z_at(buffer_coord))
    }
}

#[derive(PartialEq, Debug)]
pub struct GlCoord3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl GlCoord3D {
    pub fn new(x: f32, y: f32, z: f32) -> GlCoord3D {
        GlCoord3D { x, y, z }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct GlCoord4D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl GlCoord4D {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> GlCoord4D {
        GlCoord4D { x, y, z, w }
    }

    pub fn to_world_coord(self, transformer: &Transform) -> WorldCoord {
        transformer.unproject(self)
    }

    pub fn round(&self) -> GlCoord4D {
        GlCoord4D {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
            w: self.w.round(),
        }
    }
}

impl From<na::Point4<f32>> for GlCoord4D {
    fn from(point: na::Point4<f32>) -> Self {
        GlCoord4D {
            x: point.x,
            y: point.y,
            z: point.z,
            w: point.w,
        }
    }
}

impl From<GlCoord4D> for na::Point4<f32> {
    fn from(coord: GlCoord4D) -> Self {
        na::Point4::new(coord.x, coord.y, coord.z, coord.w)
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

    pub fn to_gl_coord_4d(self, transformer: &Transform) -> GlCoord4D {
        transformer.project(self)
    }

    pub fn to_v2_round(&self) -> V2<usize> {
        v2(self.x.round() as usize, self.y.round() as usize)
    }

    pub fn to_v2_floor(&self) -> V2<usize> {
        v2(self.x.floor() as usize, self.y.floor() as usize)
    }

    pub fn to_v2_ceil(&self) -> V2<usize> {
        v2(self.x.ceil() as usize, self.y.ceil() as usize)
    }
}

impl From<na::Point4<f32>> for WorldCoord {
    fn from(point: na::Point4<f32>) -> Self {
        WorldCoord {
            x: point.x,
            y: point.y,
            z: point.z,
        }
    }
}

impl From<WorldCoord> for na::Point4<f32> {
    fn from(coord: WorldCoord) -> Self {
        na::Point4::new(coord.x, coord.y, coord.z, 1.0)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use transform::Identity;

    #[test]
    fn physical_position_to_gl_2d_left_top() {
        let physical_size = glutin::dpi::PhysicalSize::new(100, 50);
        let physical_position = glutin::dpi::PhysicalPosition::new(0.0, 0.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GlCoord2D::new(-0.99, 0.98)
        );
    }

    #[test]
    fn physical_position_to_gl_2d_right_top() {
        let physical_size = glutin::dpi::PhysicalSize::new(100, 50);
        let physical_position = glutin::dpi::PhysicalPosition::new(100.0, 0.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GlCoord2D::new(1.01, 0.98)
        );
    }

    #[test]
    fn physical_position_to_gl_2d_left_bottom() {
        let physical_size = glutin::dpi::PhysicalSize::new(100, 50);
        let physical_position = glutin::dpi::PhysicalPosition::new(0.0, 50.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GlCoord2D::new(-0.99, -1.02)
        );
    }

    #[test]
    fn physical_position_to_gl_2d_right_bottom() {
        let physical_size = glutin::dpi::PhysicalSize::new(100, 50);
        let physical_position = glutin::dpi::PhysicalPosition::new(100.0, 50.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GlCoord2D::new(1.01, -1.02)
        );
    }

    #[test]
    fn physical_position_to_gl_2d_center() {
        let physical_size = glutin::dpi::PhysicalSize::new(100, 50);
        let physical_position = glutin::dpi::PhysicalPosition::new(50.0, 25.0);

        assert_eq!(
            physical_position.to_gl_coord_2d(physical_size),
            GlCoord2D::new(0.01, -0.02)
        );
    }

    #[test]
    fn test_physical_position_to_gl_4d() {
        let physical_size = glutin::dpi::PhysicalSize::new(100, 50);
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
            GlCoord4D::new(0.61, 0.58, 2.22, 1.0)
        );
    }

    #[test]
    fn test_gl_2d_to_buffer_coord() {
        let gl_coord_2 = GlCoord2D::new(-0.5, 0.5);
        let physical_size = glutin::dpi::PhysicalSize::new(256, 128);

        assert_eq!(
            gl_coord_2.to_buffer_coord(&physical_size),
            BufferCoordinate::new(63, 95)
        );
    }

    #[test]
    fn test_gl_2d_to_gl_4d() {
        let gl_coord_2 = GlCoord2D::new(-0.5, 0.5);
        let physical_size = glutin::dpi::PhysicalSize::new(256, 128);

        struct MockZFinder {}
        impl ZFinder for MockZFinder {
            fn get_z_at(&self, buffer_coordinate: BufferCoordinate) -> f32 {
                assert_eq!(buffer_coordinate, BufferCoordinate::new(63, 95));
                2.22
            }
        }

        assert_eq!(
            gl_coord_2.to_gl_coord_3d(&physical_size, &MockZFinder {}),
            GlCoord3D::new(-0.5, 0.5, 2.22)
        );
    }

    #[test]
    fn test_gl_4d_to_world() {
        let transform = Transform::new(
            GlCoord3D::new(1.0, 2.0, 5.0),
            GlCoord2D::new(3.0, 4.0),
            Box::new(Identity::new()),
        );

        let gl_coord_4 = GlCoord4D::new(5.0, 6.0, 7.0, 8.0);
        let expected = transform.unproject(gl_coord_4);

        assert_eq!(gl_coord_4.to_world_coord(&transform), expected);
    }

    #[test]
    fn test_gl_4d_to_na_point4() {
        let gl_coord_4: GlCoord4D = GlCoord4D::new(1.0, 2.0, 3.0, 4.0);
        let point_4: na::Point4<f32> = gl_coord_4.into();
        assert_eq!(point_4, na::Point4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_na_point4_to_gl_4d() {
        let point_4 = na::Point4::new(1.0, 2.0, 3.0, 4.0);
        let gl_coord_4: GlCoord4D = point_4.into();
        assert_eq!(gl_coord_4, GlCoord4D::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_world_to_gl_4d() {
        let transform = Transform::new(
            GlCoord3D::new(1.0, 2.0, 5.0),
            GlCoord2D::new(3.0, 4.0),
            Box::new(Identity::new()),
        );

        let world_coord = WorldCoord::new(5.0, 6.0, 7.0);
        let expected = transform.project(world_coord);

        assert_eq!(world_coord.to_gl_coord_4d(&transform), expected);
    }

    #[test]
    fn test_world_to_2d_round() {
        let world_coord = WorldCoord::new(5.9, 6.1, 7.0);

        assert_eq!(world_coord.to_v2_round(), v2(6, 6));
    }

    #[test]
    fn test_world_to_2d_floor() {
        let world_coord = WorldCoord::new(5.9, 6.1, 7.0);

        assert_eq!(world_coord.to_v2_floor(), v2(5, 6));
    }

    #[test]
    fn test_world_to_2d_ceil() {
        let world_coord = WorldCoord::new(5.9, 6.1, 7.0);

        assert_eq!(world_coord.to_v2_ceil(), v2(6, 7));
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
