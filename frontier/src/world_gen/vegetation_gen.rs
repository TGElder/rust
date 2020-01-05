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

fn get_groundwater(world: &World, position: &V2<usize>) -> f32 {
    world
        .tile_average(&position, &|cell| {
            if !world.is_sea(&cell.position) {
                Some(cell.climate.groundwater())
            } else {
                None
            }
        })
        .unwrap()
}

fn get_temperature(world: &World, position: &V2<usize>) -> f32 {
    world
        .tile_average(&position, &|cell| {
            if !world.is_sea(&cell.position) {
                Some(cell.climate.temperature)
            } else {
                None
            }
        })
        .unwrap()
}

pub fn compute_vegetation<R: Rng>(
    world: &mut World,
    params: &WorldGenParameters,
    rng: &mut R,
) -> M<WorldObject> {
    let width = world.width() - 1;
    let height = world.height() - 1;
    let mut out = M::from_element(width, height, WorldObject::None);

    let candidates = [
        VegetationType::PalmTree,
        VegetationType::DeciduousTree,
        VegetationType::EvergreenTree,
        VegetationType::Cactus,
    ];
    for x in 0..width {
        for y in 0..height {
            let position = v2(x, y);
            let max_gradient = world.get_max_abs_rise(&position);
            let min_elevation = world.get_lowest_corner(&position);
            if world.is_sea(&position)
                || max_gradient > params.cliff_gradient
                || min_elevation <= params.beach_level
            {
                continue;
            }

            let temperature = get_temperature(&world, &position);
            let groundwater = get_groundwater(&world, &position);
            let r: f32 = rng.gen_range(0.0, 1.0);
            if r <= groundwater {
                for candidate in candidates.iter() {
                    if candidate.in_range_temperature(temperature)
                        && candidate.in_range_groundwater(groundwater)
                    {
                        out[(x, y)] = WorldObject::Vegetation(*candidate);
                        break;
                    }
                }
            };
        }
    }
    out
}

pub fn load_vegetation(world: &mut World, vegetation: &M<WorldObject>) {
    for x in 0..vegetation.width() {
        for y in 0..vegetation.height() {
            world.mut_cell_unsafe(&v2(x, y)).object = vegetation[(x, y)];
        }
    }
}

fn vegetation_height_at_point(world: &World, position: &V2<usize>) -> f32 {
    world
        .get_corners_behind_in_bounds(position)
        .iter()
        .map(|corner| vegetation_height_in_cell(world, corner))
        .max_by(unsafe_ordering)
        .unwrap_or(0.0)
}

fn vegetation_height_in_cell(world: &World, position: &V2<usize>) -> f32 {
    if let WorldObject::Vegetation(vegetation) = world.get_cell_unsafe(position).object {
        vegetation.height()
    } else {
        0.0
    }
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
        world.mut_cell_unsafe(&v2(0, 0)).object = WorldObject::Vegetation(VegetationType::PalmTree);
        assert!(
            vegetation_height_at_point(&world, &v2(0, 0)).almost(VegetationType::PalmTree.height())
        );
        assert!(
            vegetation_height_at_point(&world, &v2(1, 0)).almost(VegetationType::PalmTree.height())
        );
        assert!(vegetation_height_at_point(&world, &v2(2, 0)).almost(0.0));
        assert!(
            vegetation_height_at_point(&world, &v2(0, 1)).almost(VegetationType::PalmTree.height())
        );
        assert!(vegetation_height_at_point(&world, &v2(0, 2)).almost(0.0));
        assert!(
            vegetation_height_at_point(&world, &v2(1, 1)).almost(VegetationType::PalmTree.height())
        );
    }
}
