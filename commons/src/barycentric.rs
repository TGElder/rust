use super::{v2, v3, V2, V3};

// https://gamedev.stackexchange.com/questions/23743/whats-the-most-efficient-way-to-find-barycentric-coordinates
pub fn barycentric(p: &V2<f32>, triangle: &[V2<f32>; 3]) -> V3<f32> {
    let v0 = triangle[1] - triangle[0];
    let v1 = triangle[2] - triangle[0];
    let v2 = p - triangle[0];
    let d00 = v0.dot(&v0);
    let d01 = v0.dot(&v1);
    let d11 = v1.dot(&v1);
    let d20 = v2.dot(&v0);
    let d21 = v2.dot(&v1);
    let denominator = d00 * d11 - d01 * d01;
    let v = (d11 * d20 - d01 * d21) / denominator;
    let w = (d00 * d21 - d01 * d20) / denominator;
    let u = 1.0 - v - w;
    v3(u, v, w)
}

pub fn triangle_interpolate(p: &V2<f32>, triangle: &[V3<f32>; 3]) -> Option<f32> {
    let triangle2 = [
        v2(triangle[0].x, triangle[0].y),
        v2(triangle[1].x, triangle[1].y),
        v2(triangle[2].x, triangle[2].y),
    ];
    let b = barycentric(p, &triangle2);
    if b.iter().any(|&p| p < 0.0) {
        None
    } else {
        let pz = b.x * triangle[0].z + b.y * triangle[1].z + b.z * triangle[2].z;
        Some(pz)
    }
}

pub fn triangle_interpolate_any(p: &V2<f32>, triangles: &[[V3<f32>; 3]]) -> Option<f32> {
    triangles
        .iter()
        .flat_map(|triangle| triangle_interpolate(p, triangle))
        .next()
}

#[cfg(test)]
mod tests {

    use super::*;
    use almost::Almost;

    fn triangle2() -> [V2<f32>; 3] {
        [v2(0.0, 1.0), v2(2.0, 5.0), v2(4.0, 3.0)]
    }

    fn triangle3() -> [V3<f32>; 3] {
        [v3(0.0, 1.0, 2.0), v3(2.0, 5.0, 0.0), v3(4.0, 3.0, 1.0)]
    }

    #[test]
    fn barycentric_0() {
        let actual = barycentric(&v2(0.0, 1.0), &triangle2());
        let expected = v3(1.0, 0.0, 0.0);
        assert!(actual.almost(&expected));
    }

    #[test]
    fn barycentric_1() {
        let actual = barycentric(&v2(2.0, 5.0), &triangle2());
        let expected = v3(0.0, 1.0, 0.0);
        assert!(actual.almost(&expected));
    }

    #[test]
    fn barycentric_2() {
        let actual = barycentric(&v2(4.0, 3.0), &triangle2());
        let expected = v3(0.0, 0.0, 1.0);
        assert!(actual.almost(&expected));
    }

    #[test]
    fn barycentric_between_0_and_1() {
        let actual = barycentric(&v2(1.0, 3.0), &triangle2());
        let expected = v3(0.5, 0.5, 0.0);
        assert!(actual.almost(&expected));
    }

    #[test]
    fn barycentric_between_1_and_2() {
        let actual = barycentric(&v2(3.0, 4.0), &triangle2());
        let expected = v3(0.0, 0.5, 0.5);
        assert!(actual.almost(&expected));
    }

    #[test]
    fn barycentric_between_2_and_0() {
        let actual = barycentric(&v2(2.0, 2.0), &triangle2());
        let expected = v3(0.5, 0.0, 0.5);
        assert!(actual.almost(&expected));
    }

    #[test]
    fn barycentric_centre() {
        let actual = barycentric(&v2(2.0, 3.0), &triangle2());
        let third = 1.0 / 3.0;
        let expected = v3(third, third, third);
        assert!(actual.almost(&expected));
    }

    #[test]
    fn barycentric_outside() {
        let actual = barycentric(&v2(0.0, 0.0), &triangle2());
        assert!(actual.iter().any(|&p| p < 0.0));
    }

    #[test]
    fn triangle_interpolate_0() {
        let actual = triangle_interpolate(&v2(0.0, 1.0), &triangle3());
        assert!(actual.almost(&Some(2.0)));
    }

    #[test]
    fn triangle_interpolate_1() {
        let actual = triangle_interpolate(&v2(2.0, 5.0), &triangle3());
        assert!(actual.almost(&Some(0.0)));
    }

    #[test]
    fn triangle_interpolate_2() {
        let actual = triangle_interpolate(&v2(4.0, 3.0), &triangle3());
        assert!(actual.almost(&Some(1.0)));
    }

    #[test]
    fn triangle_interpolate_between_0_and_1() {
        let actual = triangle_interpolate(&v2(1.0, 3.0), &triangle3());
        assert!(actual.almost(&Some(1.0)));
    }

    #[test]
    fn triangle_interpolate_between_1_and_2() {
        let actual = triangle_interpolate(&v2(3.0, 4.0), &triangle3());
        assert!(actual.almost(&Some(0.5)));
    }

    #[test]
    fn triangle_interpolate_between_2_and_0() {
        let actual = triangle_interpolate(&v2(2.0, 2.0), &triangle3());
        assert!(actual.almost(&Some(1.5)));
    }

    #[test]
    fn triangle_interpolate_centre() {
        let actual = triangle_interpolate(&v2(2.0, 3.0), &triangle3());
        assert!(actual.almost(&Some(1.0)));
    }

    #[test]
    fn triangle_interpolate_outside() {
        let actual = triangle_interpolate(&v2(2.0, 0.0), &triangle3());
        assert_eq!(actual, None);
    }

    #[test]
    fn test_triangle_interpolate_any() {
        let triangles = [
            [v3(0.0, 1.0, 2.0), v3(2.0, 5.0, 0.0), v3(0.0, 5.0, 1.0)],
            triangle3(),
        ];
        let actual = triangle_interpolate_any(&v2(2.0, 3.0), &triangles);
        assert!(actual.almost(&Some(1.0)));
    }
}
