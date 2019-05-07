use super::coords::*;

pub trait Projection {
    fn compute_projection_matrix(&self) -> na::Matrix4<f32>;
}

#[derive(Clone, Copy)]
pub struct Isometric {
    pub yaw: f32,
    pub pitch: f32,
}

impl Isometric {
    pub fn new(yaw: f32, pitch: f32) -> Isometric {
        Isometric { yaw, pitch }
    }
}

impl Projection for Isometric {
    #[rustfmt::skip]
    fn compute_projection_matrix(&self) -> na::Matrix4<f32> {
        let yc = self.yaw.cos();
        let ys = self.yaw.sin();
        let pc = self.pitch.cos();
        let ps = self.pitch.sin();
        na::Matrix4::from_vec(vec![
            yc, -ys, 0.0, 0.0,
            -ys * pc, -yc * pc, ps, 0.0,
            0.0, 0.0, -1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
        .transpose()
    }
}

#[allow(dead_code)]
pub struct Identity {}

#[allow(dead_code)]
impl Identity {
    pub fn new() -> Identity {
        Identity {}
    }

    pub fn boxed() -> Box<Identity> {
        Box::new(Identity::new())
    }
}

impl Projection for Identity {
    fn compute_projection_matrix(&self) -> na::Matrix4<f32> {
        na::Matrix4::identity()
    }
}

pub struct Transform {
    scale: GLCoord3D,
    translation: GLCoord2D,
    projection: Box<Projection>,
}

impl Transform {
    pub fn new(scale: GLCoord3D, translation: GLCoord2D, projection: Box<Projection>) -> Transform {
        Transform {
            scale,
            translation,
            projection,
        }
    }

    #[rustfmt::skip]
    pub fn compute_transformation_matrix(&self) -> na::Matrix4<f32> {
        let scale_matrix: na::Matrix4<f32> = na::Matrix4::from_vec(vec![
            self.scale.x, 0.0, 0.0, self.translation.x,
            0.0, self.scale.y, 0.0, self.translation.y,
            0.0, 0.0, self.scale.z, 0.0,
            0.0, 0.0, 0.0, 1.0,]
        ).transpose();

        scale_matrix * self.projection.compute_projection_matrix()
    }

    pub fn compute_inverse_matrix(&self) -> na::Matrix4<f32> {
        self.compute_transformation_matrix().try_inverse().unwrap()
    }

    #[rustfmt::skip]
    pub fn get_scale_as_matrix(&self) -> na::Matrix3<f32> {
        na::Matrix3::new(
            self.scale.x, 0.0, 0.0,
            0.0, self.scale.y, 0.0,
            0.0, 0.0, self.scale.z,
        )
    }

    pub fn translate(&mut self, delta: GLCoord2D) {
        self.translation.x = self.translation.x + delta.x;
        self.translation.y = self.translation.y + delta.y;
    }

    pub fn transform_maintaining_center(
        &mut self,
        center: GLCoord4D,
        mut transformation: Box<FnMut(&mut Self) -> ()>,
    ) {
        let old_x = center.x;
        let old_y = center.y;
        let world_point = self.unproject(center);
        transformation(self);
        let center = self.project(world_point);
        self.translation.x += old_x - center.x;
        self.translation.y += old_y - center.y;
    }

    pub fn scale(&mut self, center: GLCoord4D, delta: GLCoord2D) {
        self.transform_maintaining_center(
            center,
            Box::new(move |transform| {
                transform.scale.x = transform.scale.x * delta.x;
                transform.scale.y = transform.scale.y * delta.y;
            }),
        );
    }

    pub fn set_projection(&mut self, projection: Box<Projection>) {
        self.projection = projection
    }

    pub fn look_at(&mut self, world_coord: WorldCoord) {
        let gl_coord = world_coord.to_gl_coord_4d(self);
        self.translate(GLCoord2D::new(-gl_coord.x, -gl_coord.y));
    }

    pub fn project(&self, world_coord: WorldCoord) -> GLCoord4D {
        let point: na::Point4<f32> = world_coord.into();
        (self.compute_transformation_matrix() * point).into()
    }

    pub fn unproject(&self, projected_coord: GLCoord4D) -> WorldCoord {
        let projected_point: na::Point4<f32> = projected_coord.into();
        (self.compute_inverse_matrix() * projected_point).into()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::f32::consts::PI;

    #[rustfmt::skip]
    #[test]
    fn test_isometric_projection() {
        let isometric = Isometric::new(PI / 5.0, PI / 3.0);

        let expected  = na::Matrix4::new(
            0.809, -0.588, 0.0, 0.0,
            -0.294, -0.405, 0.866, 0.0,
            0.0, 0.0, -1.0, 0.0,
            0.0, 0.0, 0.0, 1.0
        );
        let actual = isometric.compute_projection_matrix().map(|value| (value * 1000.0).round() / 1000.0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_translate_in_constructor() {
        let transform = Transform::new(
            GLCoord3D::new(1.0, 1.0, 1.0),
            GLCoord2D::new(-1.0, 2.0),
            Identity::boxed(),
        );

        assert_eq!(
            transform.project(WorldCoord::new(0.0, 0.0, 0.0)),
            GLCoord4D::new(-1.0, 2.0, 0.0, 1.0)
        );
    }

    #[test]
    fn test_translate_method() {
        let mut transform = Transform::new(
            GLCoord3D::new(1.0, 1.0, 1.0),
            GLCoord2D::new(0.0, 0.0),
            Identity::boxed(),
        );

        transform.translate(GLCoord2D::new(-2.0, 1.0));

        assert_eq!(
            transform.project(WorldCoord::new(0.0, 0.0, 0.0)),
            GLCoord4D::new(-2.0, 1.0, 0.0, 1.0)
        );
    }

    #[test]
    fn test_get_scale_as_matrix() {
        let transform = Transform::new(
            GLCoord3D::new(2.0, 4.0, 5.0),
            GLCoord2D::new(0.0, 0.0),
            Identity::boxed(),
        );

        assert_eq!(
            transform.get_scale_as_matrix(),
            na::Matrix3::new(2.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 5.0,)
        );
    }

    #[test]
    fn test_scale_in_constructor() {
        let transform = Transform::new(
            GLCoord3D::new(2.0, 4.0, 5.0),
            GLCoord2D::new(0.0, 0.0),
            Identity::boxed(),
        );

        assert_eq!(
            transform.project(WorldCoord::new(1.0, 1.0, 1.0)),
            GLCoord4D::new(2.0, 4.0, 5.0, 1.0)
        );
    }

    #[test]
    fn test_scale_with_translation() {
        let transform = Transform::new(
            GLCoord3D::new(2.0, 4.0, 5.0),
            GLCoord2D::new(-3.0, 1.0),
            Identity::boxed(),
        );

        assert_eq!(
            transform.project(WorldCoord::new(1.0, 1.0, 1.0)),
            GLCoord4D::new(-1.0, 5.0, 5.0, 1.0)
        );
    }

    #[test]
    fn test_scale_method_center_point_should_not_change() {
        let mut transform = Transform::new(
            GLCoord3D::new(1.0, 1.0, 1.0),
            GLCoord2D::new(0.0, 0.0),
            Identity::boxed(),
        );

        transform.scale(
            GLCoord4D::new(1.0, -1.0, 7.0, 1.0),
            GLCoord2D::new(3.0, 2.0),
        );

        assert_eq!(
            transform.project(WorldCoord::new(1.0, -1.0, 7.0)),
            GLCoord4D::new(1.0, -1.0, 7.0, 1.0)
        );
    }

    #[test]
    fn test_scale_method_distance_to_other_point_should_scale() {
        let mut transform = Transform::new(
            GLCoord3D::new(1.0, 1.0, 1.0),
            GLCoord2D::new(0.0, 0.0),
            Identity::boxed(),
        );

        transform.scale(
            GLCoord4D::new(1.0, -1.0, 7.0, 1.0),
            GLCoord2D::new(3.0, 2.0),
        );

        assert_eq!(
            transform.project(WorldCoord::new(0.0, 0.0, 7.0)),
            GLCoord4D::new(-2.0, 1.0, 7.0, 1.0)
        );
    }

    #[test]
    fn test_transform_maintaining_center() {
        let mut transform = Transform::new(
            GLCoord3D::new(1.0, 1.0, 1.0),
            GLCoord2D::new(0.0, 0.0),
            Identity::boxed(),
        );

        transform.transform_maintaining_center(
            GLCoord4D::new(11.0, -17.0, 7.0, 0.0),
            Box::new(|transform: &mut Transform| {
                transform.projection = Box::new(Isometric::new(PI / 5.0, PI / 3.0))
            }),
        );

        let actual = transform.project(WorldCoord::new(11.0, -17.0, 7.0));
        // Because World Coord would have mapped to same GLCoord in identity projection

        assert_eq!(actual.x, 11.0);
        assert_eq!(actual.y, -17.0);
        // z is not maintained
    }

    #[test]
    pub fn test_look_at() {
        let mut transform = Transform::new(
            GLCoord3D::new(4.0, 1.0, 3.0),
            GLCoord2D::new(7.0, -2.0),
            Box::new(Isometric::new(PI / 5.0, PI / 3.0)),
        );
        let world_coord = WorldCoord::new(12.0, 34.0, 100.0);
        let gl_coord_4 = transform.project(world_coord);
        assert!(gl_coord_4.x != 0.0);
        assert!(gl_coord_4.y != 0.0);
        transform.look_at(world_coord);
        let gl_coord_4 = transform.project(world_coord);
        assert!(gl_coord_4.x == 0.0);
        assert!(gl_coord_4.y == 0.0);
    }

}
