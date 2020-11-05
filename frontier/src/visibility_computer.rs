extern crate line_drawing;

use commons::*;
use isometric::cell_traits::WithElevation;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::iter::once;

use line_drawing::{BresenhamCircle, Midpoint};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VisibilityComputer {
    head_height: f32,
    planet_radius: Option<f32>,
    max_distance: i64,
}

impl Default for VisibilityComputer {
    fn default() -> VisibilityComputer {
        VisibilityComputer {
            head_height: 0.002,
            planet_radius: Some(6371.0),
            max_distance: 310,
        }
    }
}

fn bresenham_cicle<'a>(
    position: &'a V2<usize>,
    radius: i64,
) -> impl Iterator<Item = (i64, i64)> + 'a {
    BresenhamCircle::new(position.x as i64, position.y as i64, radius)
}

fn bresenham_line(from: &V2<usize>, to: &(i64, i64)) -> Vec<(i64, i64)> {
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

fn to_3d<T>(grid: &dyn Grid<T>, position: V2<usize>) -> Option<V3<f32>>
where
    T: WithElevation,
{
    if let Some(cell) = grid.get_cell(&position) {
        return Some(v3(position.x as f32, position.y as f32, cell.elevation()));
    }
    None
}

fn to_position_and_3d<T>(world: &dyn Grid<T>, position: &(i64, i64)) -> Option<(V2<usize>, V3<f32>)>
where
    T: WithElevation,
{
    if let Some(position) = to_position(position) {
        if let Some(position_3d) = to_3d(world, position) {
            return Some((position, position_3d));
        }
    }
    None
}

impl VisibilityComputer {
    fn planet_curve_adjustment(&self, distance: f32) -> f32 {
        self.planet_radius
            .map(|planet_radius| {
                planet_radius - (planet_radius.powf(2.0) - distance.powf(2.0)).sqrt()
            })
            .unwrap_or(0.0)
    }

    fn check_visibility_along_line<T>(
        &self,
        world: &dyn Grid<T>,
        line: Vec<(i64, i64)>,
    ) -> Vec<V2<usize>>
    where
        T: WithElevation,
    {
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
                        to_3d.z -= self.planet_curve_adjustment(run);
                        let slope = (to_3d.z - from_3d.z) / run;
                        if slope > max_slope {
                            max_slope = slope;
                            out.push(to);
                        }
                    }
                }
            }
        }
        out
    }

    pub fn get_visible_from<T>(&self, world: &dyn Grid<T>, origin: V2<usize>) -> HashSet<V2<usize>>
    where
        T: WithElevation,
    {
        once(origin)
            .chain(
                bresenham_cicle(&origin, self.max_distance)
                    .map(|position| bresenham_line(&origin, &position))
                    .flat_map(|line| self.check_visibility_along_line(world, line)),
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::almost::Almost;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Elevation {
        elevation: f32,
    }

    impl WithElevation for Elevation {
        fn elevation(&self) -> f32 {
            self.elevation
        }
    }

    #[test]
    fn test_bresenham_circle() {
        let circle = bresenham_cicle(&v2(0, 0), 3).collect::<HashSet<(i64, i64)>>();
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
            bresenham_line(&v2(0, 0), &(3, 0)),
            vec![(0, 0), (1, 0), (2, 0), (3, 0)]
        );
        assert_eq!(
            bresenham_line(&v2(0, 0), &(3, 1)),
            vec![(0, 0), (1, 0), (2, 1), (3, 1)]
        );
        assert_eq!(
            bresenham_line(&v2(0, 0), &(2, 2)),
            vec![(0, 0), (1, 1), (2, 2)]
        );
        assert_eq!(
            bresenham_line(&v2(0, 0), &(1, 3)),
            vec![(0, 0), (0, 1), (1, 2), (1, 3)]
        );
    }

    #[test]
    fn test_run() {
        assert!(run(&v3(0.0, 0.0, 0.0), &v3(3.0, 0.0, 1.0)).almost(&3.0));
        assert!(run(&v3(0.0, 0.0, 0.0), &v3(3.0, 1.0, 1.0)).almost(&(10.0 as f32).sqrt()));
        assert!(run(&v3(0.0, 0.0, 0.0), &v3(2.0, 2.0, 1.0)).almost(&(8.0 as f32).sqrt()));
        assert!(run(&v3(0.0, 0.0, 0.0), &v3(1.0, 3.0, 1.0)).almost(&(10.0 as f32).sqrt()));
    }

    #[test]
    fn test_no_planet_curve_adjustment() {
        let visibility_computer = VisibilityComputer {
            head_height: 0.0,
            planet_radius: None,
            max_distance: 0,
        };
        assert!(visibility_computer
            .planet_curve_adjustment(100.0)
            .almost(&0.0));
    }

    #[test]
    fn test_planet_curve_adjustment() {
        let visibility_computer = VisibilityComputer {
            head_height: 0.0,
            planet_radius: Some(1000.0),
            max_distance: 0,
        };
        assert!(visibility_computer
            .planet_curve_adjustment(100.0)
            .almost(&(1000.0 - (990_000.0 as f32).sqrt())));
    }

    fn test_check_visibility_along_line(
        heights: Vec<f32>,
        expected: Vec<bool>,
        head_height: f32,
        planet_radius: Option<f32>,
    ) {
        let grid = M::from_vec(7, 1, heights).map(|elevation| Elevation { elevation });

        let visibility_computer = VisibilityComputer {
            head_height,
            planet_radius,
            max_distance: 7,
        };

        let actual_out = visibility_computer.get_visible_from(&grid, v2(0, 0));
        for (i, expected) in expected.iter().enumerate() {
            if *expected {
                assert!(actual_out.contains(&v2(i, 0)));
            }
        }
        let expected_len = expected.into_iter().filter(|t| *t).count();

        assert_eq!(actual_out.len(), expected_len);
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
        let grid = M::from_vec(
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
        )
        .map(|elevation| Elevation { elevation });

        let visibility_computer = VisibilityComputer {
            head_height: 0.0,
            planet_radius: None,
            max_distance: 3,
        };

        let out = visibility_computer.get_visible_from(&grid, v2(3, 3));

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
        let original = VisibilityComputer {
            head_height: 0.1,
            planet_radius: Some(100.0),
            max_distance: 0,
        };
        let encoded: Vec<u8> = bincode::serialize(&original).unwrap();
        let reconstructed: VisibilityComputer = bincode::deserialize(&encoded[..]).unwrap();
        assert_eq!(original, reconstructed);
    }
}
