use super::*;
use crate::world::*;
use commons::perlin::stacked_perlin_noise;
use commons::rand::prelude::*;
use commons::*;
use std::collections::HashMap;
use std::default::Default;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct VegetationParams {
    offset_range: (f32, f32),
}

impl Default for VegetationParams {
    fn default() -> VegetationParams {
        VegetationParams {
            offset_range: (0.25, 0.75),
        }
    }
}

pub struct VegetationGen<'a, R: Rng> {
    power: usize,
    world: &'a mut World,
    params: &'a WorldGenParameters,
    rng: &'a mut R,
}

impl<'a, R: Rng> VegetationGen<'a, R> {
    pub fn new(
        power: usize,
        world: &'a mut World,
        params: &'a WorldGenParameters,
        rng: &'a mut R,
    ) -> VegetationGen<'a, R> {
        VegetationGen {
            power,
            world,
            params,
            rng,
        }
    }

    pub fn compute_vegetation(&mut self) -> M<WorldObject> {
        let width = self.world.width();
        let height = self.world.height();
        let type_to_noise = self.get_vegetation_type_to_noise();
        let mut out = M::from_element(width, height, WorldObject::None);
        for x in 0..width - 1 {
            for y in 0..height - 1 {
                let position = v2(x, y);
                if !self.suitable_for_vegetation(&position) {
                    continue;
                }
                if let Some(object) = self.roll_for_vegetation(&position, &type_to_noise) {
                    out[(x, y)] = object;
                }
            }
        }
        out
    }

    fn suitable_for_vegetation(&self, position: &V2<usize>) -> bool {
        let world = &self.world;
        !world.is_sea(&position)
            && world.get_max_abs_rise(&position) <= self.params.cliff_gradient
            && world.get_lowest_corner(&position) > self.params.beach_level
    }

    fn roll_for_vegetation(
        &mut self,
        position: &V2<usize>,
        type_to_noise: &HashMap<VegetationType, M<f64>>,
    ) -> Option<WorldObject> {
        let mut candidates = vec![];
        for (vegetation_type, noise) in type_to_noise.iter() {
            if *noise.get_cell_unsafe(position) as f32 <= vegetation_type.spread()
                && self.is_candidate(*vegetation_type, position)
            {
                candidates.push(WorldObject::Vegetation {
                    vegetation_type: *vegetation_type,
                    offset: v2(self.random_offset(), self.random_offset()),
                });
            }
        }

        candidates
            .choose(self.rng)
            .filter(|_| !self.thin(position))
            .copied()
    }

    fn random_offset(&mut self) -> f32 {
        let range = self.params.vegetation.offset_range;
        self.rng.gen_range(range.0, range.1)
    }

    fn thin(&mut self, position: &V2<usize>) -> bool {
        let groundwater = unwrap_or!(self.world.tile_avg_groundwater(&position), return true);
        self.rng.gen::<f32>() > groundwater
    }

    pub fn get_vegetation_type_to_noise(&mut self) -> HashMap<VegetationType, M<f64>> {
        VEGETATION_TYPES
            .iter()
            .map(|vegetation_type| (*vegetation_type, self.get_noise(*vegetation_type)))
            .collect()
    }

    fn get_noise(&mut self, vegetation_type: VegetationType) -> M<f64> {
        let weights = get_vegetation_frequency_weights(self.power, vegetation_type);
        equalize_with_filter(
            stacked_perlin_noise(
                self.world.width(),
                self.world.height(),
                self.rng.gen(),
                weights,
            ),
            &|PositionValue { position, .. }| self.is_candidate(vegetation_type, position),
        )
    }

    fn is_candidate(&self, vegetation_type: VegetationType, position: &V2<usize>) -> bool {
        let temperature = unwrap_or!(self.world.tile_avg_temperature(&position), return false);
        let groundwater = unwrap_or!(self.world.tile_avg_groundwater(&position), return false);
        vegetation_type.in_range_temperature(temperature)
            && vegetation_type.in_range_groundwater(groundwater)
    }

    pub fn load_vegetation(&mut self, vegetation: &M<WorldObject>) {
        for x in 0..vegetation.width() {
            for y in 0..vegetation.height() {
                let position = v2(x, y);
                self.world.mut_cell_unsafe(&position).object =
                    *vegetation.get_cell_unsafe(&position);
            }
        }
    }

    pub fn set_vegetation_height(&mut self) {
        let world = &mut self.world;
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
}

fn get_vegetation_frequency_weights(power: usize, vegetation_type: VegetationType) -> Vec<f64> {
    equal_frequency_weights_starting_at(vegetation_type.clumping(), power)
}

fn equal_frequency_weights_starting_at(start_at: usize, total: usize) -> Vec<f64> {
    (0..total)
        .map(|i| if i >= start_at { 1.0 } else { 0.0 })
        .collect()
}

fn vegetation_height_at_point(world: &World, position: &V2<usize>) -> f32 {
    world
        .get_adjacent_tiles_in_bounds(position)
        .iter()
        .map(|corner| vegetation_height_in_cell(world, corner))
        .max_by(unsafe_ordering)
        .unwrap_or(0.0)
}

fn vegetation_height_in_cell(world: &World, position: &V2<usize>) -> f32 {
    if let WorldObject::Vegetation {
        vegetation_type, ..
    } = world.get_cell_unsafe(position).object
    {
        vegetation_type.height()
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::almost::Almost;

    #[test]
    pub fn test_equal_frequency_weights_starting_at_start() {
        let actual = equal_frequency_weights_starting_at(0, 5);
        let expected = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        assert_eq!(actual, expected);
    }

    #[test]
    pub fn test_equal_frequency_weights_starting_at_midway() {
        let actual = equal_frequency_weights_starting_at(3, 5);
        let expected = vec![0.0, 0.0, 0.0, 1.0, 1.0];
        assert_eq!(actual, expected);
    }

    #[test]
    pub fn test_vegetation_at() {
        let mut world = World::new(M::zeros(3, 3), 0.5);
        world.mut_cell_unsafe(&v2(0, 0)).object = WorldObject::Vegetation {
            vegetation_type: VegetationType::PalmTree,
            offset: v2(0.0, 0.0),
        };
        assert!(vegetation_height_at_point(&world, &v2(0, 0))
            .almost(&VegetationType::PalmTree.height()));
        assert!(vegetation_height_at_point(&world, &v2(1, 0))
            .almost(&VegetationType::PalmTree.height()));
        assert!(vegetation_height_at_point(&world, &v2(2, 0)).almost(&0.0));
        assert!(vegetation_height_at_point(&world, &v2(0, 1))
            .almost(&VegetationType::PalmTree.height()));
        assert!(vegetation_height_at_point(&world, &v2(0, 2)).almost(&0.0));
        assert!(vegetation_height_at_point(&world, &v2(1, 1))
            .almost(&VegetationType::PalmTree.height()));
    }
}
