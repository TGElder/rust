use super::*;
use crate::world::*;
use commons::rand::prelude::*;
use commons::*;
use std::default::Default;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct VegetationParams {}

impl Default for VegetationParams {
    fn default() -> VegetationParams {
        VegetationParams {}
    }
}

pub fn compute_vegetation<R: Rng>(
    world: &mut World,
    params: &WorldGenParameters,
    rng: &mut R,
) -> M<Vegetation> {
    let width = world.width() - 1;
    let height = world.height() - 1;
    let mut out = M::from_element(width, height, Vegetation::None);

    let candidates = [
        Vegetation::PalmTree,
        Vegetation::DeciduousTree,
        Vegetation::EvergreenTree,
        Vegetation::Cactus,
    ];
    for x in 0..width {
        for y in 0..height {
            let position = v2(x, y);
            let max_gradient = world.get_max_abs_rise(&position);
            if world.is_sea(&position) || max_gradient > params.cliff_gradient {
                continue;
            }

            let temperature = world.tile_average(&position, &|cell| cell.climate.temperature);
            let groundwater = world.tile_average(&position, &|cell| cell.climate.groundwater());
            let r: f32 = rng.gen_range(0.0, 1.0);
            if r <= groundwater {
                for candidate in candidates.iter() {
                    if candidate.in_range_temperature(temperature)
                        && candidate.in_range_groundwater(groundwater)
                    {
                        out[(x, y)] = *candidate;
                        break;
                    }
                }
            };
        }
    }
    out
}

pub fn load_vegetation(world: &mut World, vegetation: &M<Vegetation>) {
    for x in 0..vegetation.width() {
        for y in 0..vegetation.height() {
            world.mut_cell_unsafe(&v2(x, y)).climate.vegetation = vegetation[(x, y)];
        }
    }
}

fn vegetation_height_at_point(world: &World, position: &V2<usize>) -> f32 {
    world
        .get_corners_behind(position)
        .iter()
        .map(|corner| world.get_cell_unsafe(corner).climate.vegetation.height())
        .max_by(unsafe_ordering)
        .unwrap_or(0.0)
}

pub fn set_vegetation_height(world: &mut World) {
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = v2(x, y);
            let elevation = vegetation_height_at_point(&world, &position);
            world
                .mut_cell_unsafe(&position)
                .climate
                .vegetation_elevation = elevation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_vegetation_at() {
        let mut world = World::new(M::zeros(3, 3), 0.5);
        world.mut_cell_unsafe(&v2(0, 0)).climate.vegetation = Vegetation::PalmTree;
        assert_eq!(
            vegetation_height_at_point(&world, &v2(0, 0)),
            Vegetation::PalmTree.height()
        );
        assert_eq!(
            vegetation_height_at_point(&world, &v2(1, 0)),
            Vegetation::PalmTree.height()
        );
        assert_eq!(vegetation_height_at_point(&world, &v2(2, 0)), 0.0);
        assert_eq!(
            vegetation_height_at_point(&world, &v2(0, 1)),
            Vegetation::PalmTree.height()
        );
        assert_eq!(vegetation_height_at_point(&world, &v2(0, 2)), 0.0);
        assert_eq!(
            vegetation_height_at_point(&world, &v2(1, 1)),
            Vegetation::PalmTree.height()
        );
    }

}
