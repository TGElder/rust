use commons::grid::Grid;
use commons::M;

pub fn with_sea_border(terrain: M<f32>, sea_level: f32) -> M<f32> {
    clip_1_tile_in_sea_level(set_border_to_zero(terrain), sea_level)
}

fn set_border_to_zero(mut terrain: M<f32>) -> M<f32> {
    let width = terrain.width();
    let height = terrain.height();

    for x in 0..width {
        terrain[(x, 0)] = 0.0;
        terrain[(x, height - 1)] = 0.0;
    }

    for y in 0..height {
        terrain[(0, y)] = 0.0;
        terrain[(width - 1, y)] = 0.0;
    }

    terrain
}

fn clip_1_tile_in_sea_level(mut terrain: M<f32>, sea_level: f32) -> M<f32> {
    let width = terrain.width();
    let height = terrain.height();

    let clip_to_sea_level = |value: &mut f32| *value = value.min(sea_level);

    for x in 1..width - 1 {
        clip_to_sea_level(&mut terrain[(x, 1)]);
        clip_to_sea_level(&mut terrain[(x, height - 2)]);
    }

    for y in 1..height - 1 {
        clip_to_sea_level(&mut terrain[(1, y)]);
        clip_to_sea_level(&mut terrain[(width - 2, y)]);
    }

    terrain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    pub fn test_with_sea_border() {
        let terrain = M::from_element(5, 5, 1.0);
        let actual = with_sea_border(terrain, 0.5);
        let expected = M::from_vec(
            5,
            5,
            vec![
                0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.5, 0.5, 0.5, 0.0,
                0.0, 0.5, 1.0, 0.5, 0.0,
                0.0, 0.5, 0.5, 0.5, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        );
        assert_eq!(actual, expected);
    }
}
