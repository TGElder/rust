use crate::avatar::*;
use crate::world::*;
use commons::*;
use rand::prelude::*;

fn get_min_terrain_x_for_y(y: usize, world: &World) -> Option<V2<usize>> {
    (0..world.width())
        .map(|x| v2(x, y))
        .filter(|position| !world.is_sea(&position))
        .min_by(|position, other| position.x.cmp(&other.x))
}

fn get_max_terrain_x_for_y(y: usize, world: &World) -> Option<V2<usize>> {
    (0..world.width())
        .map(|x| v2(x, y))
        .filter(|position| !world.is_sea(&position))
        .max_by(|position, other| position.x.cmp(&other.x))
}

fn get_min_terrain_y_for_x(x: usize, world: &World) -> Option<V2<usize>> {
    (0..world.height())
        .map(|y| v2(x, y))
        .filter(|position| !world.is_sea(&position))
        .min_by(|position, other| position.x.cmp(&other.x))
}

fn get_max_terrain_y_for_x(x: usize, world: &World) -> Option<V2<usize>> {
    (0..world.height())
        .map(|y| v2(x, y))
        .filter(|position| !world.is_sea(&position))
        .max_by(|position, other| position.x.cmp(&other.x))
}

fn get_left_candidates(distance: i64, world: &World) -> Vec<ShoreStart> {
    (0..world.height())
        .map(|y| get_min_terrain_x_for_y(y, world))
        .filter_map(|position| position)
        .map(|landfall| ShoreStart {
            origin: v2(0, landfall.y),
            landfall,
            at: world.clip_to_in_bounds(&v2(landfall.x as i64 - distance, landfall.y as i64)),
            rotation: Rotation::Right,
        })
        .collect()
}

fn get_right_candidates(distance: i64, world: &World) -> Vec<ShoreStart> {
    (0..world.height())
        .map(|y| get_max_terrain_x_for_y(y, world))
        .filter_map(|position| position)
        .map(|landfall| ShoreStart {
            origin: v2(world.width() - 1, landfall.y),
            at: world.clip_to_in_bounds(&v2(landfall.x as i64 + distance, landfall.y as i64)),
            landfall,
            rotation: Rotation::Left,
        })
        .collect()
}

fn get_top_candidates(distance: i64, world: &World) -> Vec<ShoreStart> {
    (0..world.width())
        .map(|x| get_min_terrain_y_for_x(x, world))
        .filter_map(|position| position)
        .map(|landfall| ShoreStart {
            origin: v2(landfall.x, 0),
            at: world.clip_to_in_bounds(&v2(landfall.x as i64, landfall.y as i64 - distance)),
            landfall,
            rotation: Rotation::Up,
        })
        .collect()
}

fn get_bottom_candidates(distance: i64, world: &World) -> Vec<ShoreStart> {
    (0..world.width())
        .map(|x| get_max_terrain_y_for_x(x, world))
        .filter_map(|position| position)
        .map(|landfall| ShoreStart {
            origin: v2(landfall.x, world.height() - 1),
            at: world.clip_to_in_bounds(&v2(landfall.x as i64, landfall.y as i64 + distance)),
            landfall,
            rotation: Rotation::Down,
        })
        .collect()
}

fn get_candidates(distance: i64, world: &World) -> Vec<ShoreStart> {
    let mut out = vec![];
    out.append(&mut get_left_candidates(distance, world));
    out.append(&mut get_right_candidates(distance, world));
    out.append(&mut get_bottom_candidates(distance, world));
    out.append(&mut get_top_candidates(distance, world));
    out
}

pub fn shore_start<R: Rng>(distance: i64, world: &World, rng: &mut R) -> ShoreStart {
    *get_candidates(distance, world)
        .choose(rng)
        .expect("No suitable starting positions!")
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ShoreStart {
    origin: V2<usize>,
    at: V2<usize>,
    landfall: V2<usize>,
    rotation: Rotation,
}

impl ShoreStart {
    pub fn origin(&self) -> V2<usize> {
        self.origin
    }

    pub fn at(&self) -> V2<usize> {
        self.at
    }

    #[allow(dead_code)]
    pub fn landfall(&self) -> V2<usize> {
        self.landfall
    }

    pub fn rotation(&self) -> Rotation {
        self.rotation
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::M;

    #[rustfmt::skip]
    fn world() -> World {
        World::new(
            M::from_vec(5, 5, 
            vec![
                0.0, 0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 1.0, 0.0,
                0.0, 1.0, 1.0, 1.0, 0.0,
                0.0, 1.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
            0.5,
        )
    }

    #[test]
    fn test_get_min_terrain_x_for_y() {
        let world = &world();
        assert_eq!(get_min_terrain_x_for_y(0, world), Some(v2(2, 0)));
        assert_eq!(get_min_terrain_x_for_y(1, world), Some(v2(2, 1)));
        assert_eq!(get_min_terrain_x_for_y(2, world), Some(v2(1, 2)));
        assert_eq!(get_min_terrain_x_for_y(3, world), Some(v2(1, 3)));
        assert_eq!(get_min_terrain_x_for_y(4, world), None);
    }

    #[test]
    fn test_get_max_terrain_x_for_y() {
        let world = &world();
        assert_eq!(get_max_terrain_x_for_y(0, world), Some(v2(2, 0)));
        assert_eq!(get_max_terrain_x_for_y(1, world), Some(v2(3, 1)));
        assert_eq!(get_max_terrain_x_for_y(2, world), Some(v2(3, 2)));
        assert_eq!(get_max_terrain_x_for_y(3, world), Some(v2(1, 3)));
        assert_eq!(get_max_terrain_x_for_y(4, world), None);
    }

    #[test]
    fn test_get_min_terrain_y_for_x() {
        let world = &world();
        assert_eq!(get_min_terrain_y_for_x(0, world), None);
        assert_eq!(get_min_terrain_y_for_x(1, world), Some(v2(1, 2)));
        assert_eq!(get_min_terrain_y_for_x(2, world), Some(v2(2, 0)));
        assert_eq!(get_min_terrain_y_for_x(3, world), Some(v2(3, 1)));
        assert_eq!(get_min_terrain_y_for_x(4, world), None);
    }

    #[test]
    fn test_get_max_terrain_y_for_x() {
        let world = &world();
        assert_eq!(get_max_terrain_y_for_x(0, world), None);
        assert_eq!(get_max_terrain_y_for_x(1, world), Some(v2(1, 3)));
        assert_eq!(get_max_terrain_y_for_x(2, world), Some(v2(2, 2)));
        assert_eq!(get_max_terrain_y_for_x(3, world), Some(v2(3, 2)));
        assert_eq!(get_max_terrain_y_for_x(4, world), None);
    }

    #[test]
    fn test_get_left_candidates() {
        let world = &world();
        assert_eq!(
            get_left_candidates(1, world),
            vec![
                ShoreStart {
                    origin: v2(0, 0),
                    at: v2(1, 0),
                    landfall: v2(2, 0),
                    rotation: Rotation::Right,
                },
                ShoreStart {
                    origin: v2(0, 1),
                    at: v2(1, 1),
                    landfall: v2(2, 1),
                    rotation: Rotation::Right,
                },
                ShoreStart {
                    origin: v2(0, 2),
                    at: v2(0, 2),
                    landfall: v2(1, 2),
                    rotation: Rotation::Right,
                },
                ShoreStart {
                    origin: v2(0, 3),
                    at: v2(0, 3),
                    landfall: v2(1, 3),
                    rotation: Rotation::Right,
                },
            ]
        );
    }

    #[test]
    fn test_get_right_candidates() {
        let world = &world();
        assert_eq!(
            get_right_candidates(1, world),
            vec![
                ShoreStart {
                    origin: v2(4, 0),
                    at: v2(3, 0),
                    landfall: v2(2, 0),
                    rotation: Rotation::Left,
                },
                ShoreStart {
                    origin: v2(4, 1),
                    at: v2(4, 1),
                    landfall: v2(3, 1),
                    rotation: Rotation::Left,
                },
                ShoreStart {
                    origin: v2(4, 2),
                    at: v2(4, 2),
                    landfall: v2(3, 2),
                    rotation: Rotation::Left,
                },
                ShoreStart {
                    origin: v2(4, 3),
                    at: v2(2, 3),
                    landfall: v2(1, 3),
                    rotation: Rotation::Left,
                },
            ]
        );
    }

    #[test]
    fn test_get_top_candidates() {
        let world = &world();
        assert_eq!(
            get_top_candidates(1, world),
            vec![
                ShoreStart {
                    origin: v2(1, 0),
                    at: v2(1, 1),
                    landfall: v2(1, 2),
                    rotation: Rotation::Up,
                },
                ShoreStart {
                    origin: v2(2, 0),
                    at: v2(2, 0),
                    landfall: v2(2, 0),
                    rotation: Rotation::Up,
                },
                ShoreStart {
                    origin: v2(3, 0),
                    at: v2(3, 0),
                    landfall: v2(3, 1),
                    rotation: Rotation::Up,
                },
            ]
        );
    }

    #[test]
    fn test_get_bottom_candidates() {
        let world = &world();
        assert_eq!(
            get_bottom_candidates(1, world),
            vec![
                ShoreStart {
                    origin: v2(1, 4),
                    at: v2(1, 4),
                    landfall: v2(1, 3),
                    rotation: Rotation::Down,
                },
                ShoreStart {
                    origin: v2(2, 4),
                    at: v2(2, 3),
                    landfall: v2(2, 2),
                    rotation: Rotation::Down,
                },
                ShoreStart {
                    origin: v2(3, 4),
                    at: v2(3, 3),
                    landfall: v2(3, 2),
                    rotation: Rotation::Down,
                },
            ]
        );
    }
}
