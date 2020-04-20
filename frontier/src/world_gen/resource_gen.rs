use super::*;
use crate::world::*;
use commons::rand::prelude::*;
use commons::*;
use std::default::Default;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FarmlandConstraints {
    pub min_groundwater: f32,
    pub max_slope: f32,
    pub min_temperature: f32,
}

impl Default for FarmlandConstraints {
    fn default() -> FarmlandConstraints {
        FarmlandConstraints {
            min_groundwater: 0.1,
            max_slope: 0.2,
            min_temperature: 0.0,
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ResourceParams {
    farmland: FarmlandConstraints,
}

impl Default for ResourceParams {
    fn default() -> ResourceParams {
        ResourceParams {
            farmland: FarmlandConstraints::default(),
        }
    }
}

pub struct ResourceGen<'a, R: Rng> {
    world: &'a mut World,
    params: &'a WorldGenParameters,
    rng: &'a mut R,
}

impl<'a, R: Rng> ResourceGen<'a, R> {
    pub fn new(
        world: &'a mut World,
        params: &'a WorldGenParameters,
        rng: &'a mut R,
    ) -> ResourceGen<'a, R> {
        ResourceGen { world, params, rng }
    }

    pub fn compute_resources(&mut self) -> M<Resource> {
        let width = self.world.width() - 1;
        let height = self.world.height() - 1;
        let mut out = M::from_element(width, height, Resource::None);

        // Otherwise probability one resources dominate
        let mut resources_by_probability = RESOURCES;
        resources_by_probability
            .sort_by(|&a, &b| probability(a).partial_cmp(&probability(b)).unwrap());

        for x in 0..width {
            for y in 0..height {
                let position = v2(x, y);
                if self.world.is_sea(&position) {
                    continue;
                }
                let resources: Vec<Resource> = resources_by_probability
                    .iter()
                    .cloned()
                    .filter(|&resource| self.is_candidate(resource, &position))
                    .collect();
                if let Some(resource) = self.get_random_resource(&resources) {
                    out[(x, y)] = resource;
                }
            }
        }
        out
    }

    pub fn load_resources(&mut self, resources: &M<Resource>) {
        for x in 0..resources.width() {
            for y in 0..resources.height() {
                self.world.mut_cell_unsafe(&v2(x, y)).resource = resources[(x, y)];
            }
        }
    }

    fn get_random_resource(&mut self, resources: &[Resource]) -> Option<Resource> {
        let r = self.rng.gen_range(0.0, 1.0);
        let mut cum = 0.0;
        for resource in resources {
            cum += probability(*resource);
            if r <= cum {
                return Some(*resource);
            }
        }
        None
    }

    fn is_candidate(&self, resource: Resource, position: &V2<usize>) -> bool {
        if self.is_beach(position) {
            return false;
        }
        match resource {
            Resource::Bananas => self.has_vegetation_type(position, VegetationType::PalmTree),
            Resource::Coal => self.is_cliff(position),
            Resource::Deer => self.has_vegetation_type(position, VegetationType::DeciduousTree),
            Resource::Farmland => self.is_farmland_candidate(position),
            Resource::Fur => self.has_vegetation_type(position, VegetationType::EvergreenTree),
            Resource::Gems => true,
            Resource::Gold => self.by_river(position),
            Resource::Iron => self.is_cliff(position),
            Resource::Ivory => {
                !self.is_cliff(position)
                    && self.among_vegetation_type(position, VegetationType::PalmTree)
            }
            Resource::Spice => self.has_vegetation_type(position, VegetationType::PalmTree),
            Resource::Stone => self.is_cliff(position),
            _ => false,
        }
    }

    fn has_vegetation_type(&self, position: &V2<usize>, vegetation_type: VegetationType) -> bool {
        match self.world.get_cell(position) {
            Some(WorldCell {
                object: WorldObject::Vegetation(actual),
                ..
            }) if *actual == vegetation_type => true,
            _ => false,
        }
    }

    fn among_vegetation_type(&self, position: &V2<usize>, vegetation_type: VegetationType) -> bool {
        match self.world.get_cell(position) {
            Some(WorldCell {
                object: WorldObject::None,
                ..
            }) => (),
            _ => return false,
        };
        match self.world.tile_avg_temperature(&position) {
            Some(temperature) if vegetation_type.in_range_temperature(temperature) => (),
            _ => return false,
        };
        match self.world.tile_avg_groundwater(&position) {
            Some(groundwater) if vegetation_type.in_range_groundwater(groundwater) => (),
            _ => return false,
        };
        true
    }

    fn is_beach(&self, position: &V2<usize>) -> bool {
        self.world.get_lowest_corner(&position) <= self.params.beach_level
    }

    fn is_cliff(&self, position: &V2<usize>) -> bool {
        self.world.get_max_abs_rise(&position) > self.params.cliff_gradient
    }

    fn is_farmland_candidate(&self, position: &V2<usize>) -> bool {
        !self.is_cliff(position)
            && (self.among_vegetation_type(position, VegetationType::EvergreenTree)
                || self.among_vegetation_type(position, VegetationType::DeciduousTree)
                || self.among_vegetation_type(position, VegetationType::PalmTree))
    }

    fn by_river(&self, position: &V2<usize>) -> bool {
        self.world
            .get_border(position)
            .iter()
            .any(|edge| self.world.is_river(edge))
    }
}

fn probability(resource: Resource) -> f32 {
    match resource {
        Resource::Bananas => 1.0 / 512.0,
        Resource::Coal => 1.0 / 1024.0,
        Resource::Deer => 1.0 / 512.0,
        Resource::Farmland => 1.0,
        Resource::Fur => 1.0 / 512.0,
        Resource::Gems => 1.0 / 8192.0,
        Resource::Gold => 1.0 / 2048.0,
        Resource::Iron => 1.0 / 2048.0,
        Resource::Ivory => 1.0 / 2048.0,
        Resource::Spice => 1.0 / 512.0,
        Resource::Stone => 1.0 / 256.0,
        _ => 0.0,
    }
}
