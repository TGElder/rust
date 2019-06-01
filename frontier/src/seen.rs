extern crate line_drawing;

use crate::avatar::*;
use crate::world::*;
use commons::{v2, v3, M, V2, V3};
use isometric::coords::*;
use serde::{Deserialize, Serialize};

use line_drawing::{BresenhamCircle, Midpoint};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Seen {
    visited: M<bool>,
    head_height: f32,
    planet_radius: Option<f32>,
}

fn bresenham_cicle(position: &V2<usize>, radius: i64) -> Vec<(i64, i64)> {
    BresenhamCircle::new(position.x as i64, position.y as i64, radius).collect()
}

fn bresenham_line(from: &V2<usize>, to: (i64, i64)) -> Vec<(i64, i64)> {
    Midpoint::new((from.x as f32, from.y as f32), (to.0 as f32, to.1 as f32)).collect()
}

fn run(from: &V3<f32>, to: &V3<f32>) -> f32 {
    ((from.x - to.x).powf(2.0) + (from.y - to.y).powf(2.0)).sqrt()
}

fn to_position(position: &(i64, i64)) -> Option<V2<usize>> {
    if position.0 >= 0 && position.1 >= 0 {
        Some(v2(position.0 as usize, position.1 as usize))
    } else {
        None
    }
}

fn to_3d(world: &World, position: V2<usize>) -> Option<V3<f32>> {
    if let Some(z) = world.get_elevation(&position) {
        return Some(v3(
            position.x as f32,
            position.y as f32,
            z.max(world.sea_level()),
        ));
    }
    return None;
}

fn to_position_and_3d(world: &World, position: &(i64, i64)) -> Option<(V2<usize>, V3<f32>)> {
    if let Some(position) = to_position(position) {
        if let Some(position_3d) = to_3d(world, position) {
            return Some((position, position_3d));
        }
    }
    return None;
}

impl Seen {
    pub fn new(world: &World, head_height: f32, planet_radius: Option<f32>) -> Seen {
        Seen {
            visited: M::from_element(world.width(), world.height(), false),
            head_height,
            planet_radius,
        }
    }

    pub fn point_has_been_visited(&self, position: &V2<usize>) -> bool {
        self.visited[(position.x, position.y)]
    }

    pub fn set_visited(&mut self, position: &V2<usize>) {
        self.visited[(position.x, position.y)] = true;
    }

    fn planet_curve_adjustment(&self, distance: f32) -> f32 {
        self.planet_radius
            .map(|planet_radius| {
                planet_radius - (planet_radius.powf(2.0) - distance.powf(2.0)).sqrt()
            })
            .unwrap_or(0.0)
    }

    fn check_visibility_along_line(&self, world: &World, line: Vec<(i64, i64)>) -> Vec<V2<usize>> {
        let mut max_slope = -std::f32::INFINITY;
        let mut out = vec![];
        if let Some((from, mut from_3d)) = to_position_and_3d(world, &line[0]) {
            from_3d.z += self.head_height;
            out.push(from);
            for position in line.iter().skip(1) {
                match to_position_and_3d(world, position) {
                    None => return out,
                    Some((to, mut to_3d)) => {
                        let run = run(&from_3d, &to_3d);
                        to_3d.z = to_3d.z - self.planet_curve_adjustment(run);
                        let slope = (to_3d.z - from_3d.z) / run;
                        if slope > max_slope {
                            max_slope = slope;
                            out.push(to);
                        }
                    }
                }
            }
        }
        return out;
    }

    pub fn update_visibility(
        &mut self,
        world: &mut World,
        avatar: &Avatar,
        max_distance: i64,
    ) -> Vec<V2<usize>> {
        let mut out = vec![];
        if let Some(WorldCoord { x, y, .. }) = avatar.compute_world_coord(world) {
            let origin = v2(x.round() as usize, y.round() as usize);
            if self.point_has_been_visited(&origin) {
                return vec![];
            } else {
                self.set_visited(&origin);
            }
            world.set_visible(&origin);
            out.push(origin);
            for position in bresenham_cicle(&origin, max_distance) {
                let line = bresenham_line(&origin, position);
                let mut visible = self.check_visibility_along_line(world, line);
                visible.retain(|position| !world.is_visible(&position));
                visible
                    .iter()
                    .for_each(|position| world.set_visible(position));
                out.append(&mut visible);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Instant;

    fn world() -> World {
        World::new(
            M::from_vec(1, 1, vec![0.0]),
            vec![],
            vec![],
            0.0,
            Instant::now(),
        )
    }

    #[test]
    fn test_bresenham_circle() {
        let circle = bresenham_cicle(&v2(0, 0), 3);
        assert!(circle.contains(&(-3, 0)));
        assert!(circle.contains(&(-3, 1)));
        assert!(circle.contains(&(-2, 2)));
        assert!(circle.contains(&(-1, 3)));
        assert!(circle.contains(&(0, 3)));
        assert!(circle.contains(&(1, 3)));
        assert!(circle.contains(&(2, 2)));
        assert!(circle.contains(&(3, 1)));
        assert!(circle.contains(&(3, 0)));
        assert!(circle.contains(&(3, -1)));
        assert!(circle.contains(&(2, -2)));
        assert!(circle.contains(&(1, -3)));
        assert!(circle.contains(&(0, -3)));
        assert!(circle.contains(&(-1, -3)));
        assert!(circle.contains(&(-2, -2)));
        assert!(circle.contains(&(-3, -1)));
        assert_eq!(circle.len(), 16);
    }

    #[test]
    fn test_bresenham_line() {
        assert_eq!(
            bresenham_line(&v2(0, 0), (3, 0)),
            vec![(0, 0), (1, 0), (2, 0), (3, 0)]
        );
        assert_eq!(
            bresenham_line(&v2(0, 0), (3, 1)),
            vec![(0, 0), (1, 0), (2, 1), (3, 1)]
        );
        assert_eq!(
            bresenham_line(&v2(0, 0), (2, 2)),
            vec![(0, 0), (1, 1), (2, 2)]
        );
        assert_eq!(
            bresenham_line(&v2(0, 0), (1, 3)),
            vec![(0, 0), (0, 1), (1, 2), (1, 3)]
        );
    }

    #[test]
    fn test_run() {
        assert_eq!(run(&v3(0.0, 0.0, 0.0), &v3(3.0, 0.0, 1.0)), 3.0);
        assert_eq!(
            run(&v3(0.0, 0.0, 0.0), &v3(3.0, 1.0, 1.0)),
            (10.0 as f32).sqrt()
        );
        assert_eq!(
            run(&v3(0.0, 0.0, 0.0), &v3(2.0, 2.0, 1.0)),
            (8.0 as f32).sqrt()
        );
        assert_eq!(
            run(&v3(0.0, 0.0, 0.0), &v3(1.0, 3.0, 1.0)),
            (10.0 as f32).sqrt()
        );
    }

    #[test]
    fn test_no_planet_curve_adjustment() {
        let seen = Seen::new(&world(), 0.0, None);
        assert_eq!(seen.planet_curve_adjustment(100.0), 0.0);
    }

    #[test]
    fn test_planet_curve_adjustment() {
        let seen = Seen::new(&world(), 0.0, Some(1000.0));
        assert_eq!(
            seen.planet_curve_adjustment(100.0),
            1000.0 - (990000.0 as f32).sqrt()
        );
    }

    fn test_check_visibility_along_line(
        heights: Vec<f32>,
        expected: Vec<bool>,
        head_height: f32,
        planet_curve: Option<f32>,
    ) {
        let mut world = World::new(
            M::from_vec(7, 1, heights),
            vec![],
            vec![],
            0.0,
            Instant::now(),
        );

        let mut seen = Seen::new(&world, head_height, planet_curve);
        let mut avatar = Avatar::new(0.0);
        avatar.reposition(v2(0, 0), Rotation::Up);

        let actual_out = seen.update_visibility(&mut world, &avatar, 7);
        let mut expected_out = vec![];
        for x in 0..expected.len() {
            assert_eq!(
                format!("{},0 = {}", x, world.is_visible(&v2(x, 0))),
                format!("{},0 = {}", x, expected[x]),
            );
            if expected[x] {
                expected_out.push(v2(x, 0));
            }
        }

        assert_eq!(actual_out, expected_out);
    }

    #[test]
    fn test_check_visibility_along_line_flat() {
        test_check_visibility_along_line(
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![true, true, false, false, false, false, false],
            0.0,
            None,
        );
    }

    #[test]
    fn test_check_visibility_along_line_dip() {
        test_check_visibility_along_line(
            vec![2.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0],
            vec![true, true, true, true, true, false, false],
            0.0,
            None,
        );
    }

    #[test]
    fn test_check_visibility_along_line_hill() {
        test_check_visibility_along_line(
            vec![0.0, 1.0, 3.0, 1.0, 0.0, 0.0, 0.0],
            vec![true, true, true, false, false, false, false],
            0.0,
            None,
        );
    }

    #[test]
    fn test_check_visibility_along_line_hill_behind_hill() {
        test_check_visibility_along_line(
            vec![0.0, 1.0, 3.0, 1.0, 10.0, 1.0, 0.0],
            vec![true, true, true, false, true, false, false],
            0.0,
            None,
        );
    }

    #[test]
    fn test_check_visibility_along_line_flat_with_raised_head() {
        test_check_visibility_along_line(
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![true, true, true, true, true, true, true],
            0.01,
            None,
        );
    }

    #[test]
    fn test_check_visibility_along_line_flat_with_raised_head_and_curve() {
        test_check_visibility_along_line(
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![true, true, true, true, true, false, false],
            0.01,
            Some(1000.0),
        );
    }

    #[test]
    #[rustfmt::skip]
    fn update_visibility() {
        let mut world = World::new(
            M::from_vec(
                7,
                7,
                vec![
                    8.0, 8.0, 2.0, 1.0, 1.0, 8.0, 8.0, 
                    8.0, 1.0, 2.0, 1.0, 1.0, 1.0, 8.0, 
                    8.0, 1.0, 2.0, 0.0, 0.0, 0.0, 1.0, 
                    8.0, 1.0, 2.0, 1.0, 1.0, 0.0, 1.0, 
                    8.0, 1.0, 2.0, 1.0, 1.0, 0.0, 1.0,
                    8.0, 1.0, 2.0, 1.0, 1.0, 0.0, 8.0, 
                    8.0, 8.0, 2.0, 1.0, 1.0, 8.0, 8.0,
                ],
            ),
            vec![],
            vec![],
            0.5,
            Instant::now(),
        );

        let mut avatar = Avatar::new(0.0);
        avatar.reposition(v2(3, 3), Rotation::Up);

        let mut seen = Seen::new(&world, 0.0, None);

        let out = seen.update_visibility(&mut world, &avatar, 3);

        let expected = M::from_vec(
            7,
            7,
            vec![
                false, false, false, false, false, false, false, 
                false, false, true , true , true , true , false, 
                true , false, true , true , true , false, false, 
                true , false, true , true , true , false, false, 
                true , false, true , true , true , false, false, 
                false, false, true , false, false, false, false, 
                false, false, false, false, false, false, false,
            ],
        );

        for x in 0..7 {
            for y in 0..7 {
                assert_eq!(world.is_visible(&v2(x, y)), expected[(x, y)]);
            }
        }

        assert_eq!(out.len(), 17);

        assert!(out.contains(&v2(2, 1)));
        assert!(out.contains(&v2(3, 1)));
        assert!(out.contains(&v2(4, 1)));
        assert!(out.contains(&v2(5, 1)));
        assert!(out.contains(&v2(0, 2)));
        assert!(out.contains(&v2(2, 2)));
        assert!(out.contains(&v2(3, 2)));
        assert!(out.contains(&v2(4, 2)));
        assert!(out.contains(&v2(0, 3)));
        assert!(out.contains(&v2(2, 3)));
        assert!(out.contains(&v2(3, 3)));
        assert!(out.contains(&v2(4, 3)));
        assert!(out.contains(&v2(0, 4)));
        assert!(out.contains(&v2(2, 4)));
        assert!(out.contains(&v2(3, 4)));
        assert!(out.contains(&v2(4, 4)));
        assert!(out.contains(&v2(2, 5)));
    }

    #[test]
    fn round_trip() {
        let original = Seen::new(&world(), 0.1, Some(100.0));
        let encoded: Vec<u8> = bincode::serialize(&original).unwrap();
        let reconstructed: Seen = bincode::deserialize(&encoded[..]).unwrap();
        assert_eq!(original, reconstructed);
    }

}
