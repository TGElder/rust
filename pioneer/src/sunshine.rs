use commons::scale::*;
use commons::*;

fn get_normal(elevations: &M<f64>, position: &V2<usize>) -> V3<f64> {
    let x1 = elevations.offset(position, &v2(-1, 0)).unwrap_or(*position);
    let x2 = elevations.offset(position, &v2(1, 0)).unwrap_or(*position);
    let y1 = elevations.offset(position, &v2(0, -1)).unwrap_or(*position);
    let y2 = elevations.offset(position, &v2(0, 1)).unwrap_or(*position);

    let to_vector_3d = |position: &V2<usize>| {
        v3(
            position.x as f64,
            position.y as f64,
            *elevations.get_cell_unsafe(position),
        )
    };

    let x = to_vector_3d(&x2) - to_vector_3d(&x1);
    let y = to_vector_3d(&y2) - to_vector_3d(&y1);

    x.cross(&y)
}

fn get_angle_to_sun(slope_normal: &V3<f64>, sun_direction: &V3<f64>) -> f64 {
    slope_normal.angle(&(sun_direction * -1.0))
}

fn sun_direction_at_latitude(latitude: f64) -> V3<f64> {
    let radians = latitude.to_radians();
    v3(0.0, radians.sin(), -radians.cos())
}

fn sunshine_at(elevations: &M<f64>, position: &V2<usize>, y_to_latitude: &Scale<f64>) -> f64 {
    let latitude = y_to_latitude.scale(position.y as f64) as f64;
    let sun_direction = sun_direction_at_latitude(latitude);
    let normal = get_normal(elevations, position);
    get_angle_to_sun(&normal, &sun_direction)
}

pub fn sunshine(elevations: &M<f64>, y_to_latitude: &Scale<f64>) -> M<f64> {
    M::from_fn(elevations.width(), elevations.height(), |x, y| {
        sunshine_at(elevations, &v2(x, y), y_to_latitude)
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::f64::consts::PI;

    #[rustfmt::skip]
    #[test]
    fn test_get_normal_90_degree() {
        let elevations = M::from_vec(3, 3, vec![
            0.0, 1.0, 0.0,
            2.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ]);
        assert_eq!(
            get_normal(&elevations, &v2(1, 1)).normalize(),
            v3(1.0, 0.0, 1.0).normalize()
        );
    }

    #[rustfmt::skip]
    #[test]
    fn test_corner() {
        let elevations = M::from_vec(2, 2, vec![
            0.0, 1.0, 
            1.0, 2.0
        ]);
        assert_eq!(
            get_normal(&elevations, &v2(0, 0)).normalize(),
            v3(-1.0, -1.0, 1.0).normalize()
        );
    }

    #[rustfmt::skip]
    #[test]
    fn test_get_normal_flat() {
        let elevations = M::from_vec(3, 3, vec![
            0.0, 0.0, 0.0,
            0.0, 0.0, 0.0,
            0.0, 0.0, 0.0,
        ]);
        assert_eq!(
            get_normal(&elevations, &v2(1, 1)).normalize(),
            v3(0.0, 0.0, 1.0)
        );
    }

    #[test]
    fn test_get_angle_to_sun() {
        almost_equal(
            get_angle_to_sun(&v3(0.0, 0.0, 1.0), &v3(0.0, 0.0, -1.0)),
            0.0,
        );
        almost_equal(
            get_angle_to_sun(&v3(0.0, 0.0, 1.0), &v3(-1.0, 0.0, 0.0)),
            PI / 2.0,
        );
        almost_equal(
            get_angle_to_sun(&v3(0.0, 0.0, 1.0), &v3(-1.0, 0.0, -1.0)),
            PI / 4.0,
        );
        almost_equal(
            get_angle_to_sun(&v3(1.0, 0.0, 1.0), &v3(-1.0, 0.0, -1.0)),
            0.0,
        );
        almost_equal(
            get_angle_to_sun(&v3(1.0, 0.0, 1.0), &v3(1.0, 0.0, -1.0)),
            PI / 2.0,
        );
        almost_equal(
            get_angle_to_sun(&v3(1.0, 0.0, 1.0), &v3(1.0, 0.0, 0.0)),
            3.0 * PI / 4.0,
        );
        almost_equal(
            get_angle_to_sun(&v3(1.0, 0.0, 1.0), &v3(0.0, 1.0, 0.0)),
            PI / 2.0,
        );
    }

    #[test]
    fn test_sun_direction_at_latitude() {
        all_almost_equal(sun_direction_at_latitude(90.0), v3(0.0, 1.0, 0.0));
        all_almost_equal(
            sun_direction_at_latitude(45.0),
            v3(0.0, 1.0, -1.0).normalize(),
        );
        all_almost_equal(sun_direction_at_latitude(0.0), v3(0.0, 0.0, -1.0));
        all_almost_equal(
            sun_direction_at_latitude(-45.0),
            v3(0.0, -1.0, -1.0).normalize(),
        );
        all_almost_equal(sun_direction_at_latitude(-90.0), v3(0.0, -1.0, 0.0));
    }

    fn almost_equal(a: f64, b: f64) {
        let epsilon = 0.000001;
        assert!((a - b).abs() < epsilon)
    }

    fn all_almost_equal(a: V3<f64>, b: V3<f64>) {
        almost_equal(a.x, b.x);
        almost_equal(a.y, b.y);
        almost_equal(a.z, b.z);
    }

}
