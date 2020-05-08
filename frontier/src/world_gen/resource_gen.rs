use super::*;
use crate::world::*;
use commons::rand::prelude::*;
use commons::rand::seq::SliceRandom;
use commons::*;
use std::collections::{HashMap, HashSet};
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
        let width = self.world.width();
        let height = self.world.height();
        let mut out = M::from_element(width, height, Resource::None);
        self.add_limited_resources(&mut out);
        self.add_unlimited_resources(&mut out);
        out
    }

    fn add_limited_resources(&mut self, resources: &mut M<Resource>) {
        let mut taken: HashSet<V2<usize>> = HashSet::new();
        for (resource, mut candidates) in self.get_candidates() {
            candidates.retain(|candidate| !taken.contains(candidate));
            if let Some(count) = count(resource) {
                let chosen: Vec<&V2<usize>> =
                    candidates.choose_multiple(&mut self.rng, count).collect();
                chosen
                    .iter()
                    .for_each(|position| *resources.mut_cell_unsafe(position) = resource);
                taken.extend(chosen);
            }
        }
    }

    fn get_candidates(&self) -> HashMap<Resource, Vec<V2<usize>>> {
        let width = self.world.width();
        let height = self.world.height();

        let mut out = HashMap::new();

        for x in 0..width {
            for y in 0..height {
                let position = v2(x, y);
                if self.world.is_sea(&position) {
                    continue;
                }
                RESOURCES
                    .iter()
                    .filter(|&resource| {
                        count(*resource).is_some() && self.is_candidate(*resource, &position)
                    })
                    .for_each(|resource| {
                        out.entry(*resource)
                            .or_insert_with(|| vec![])
                            .push(position)
                    });
            }
        }

        out
    }

    fn add_unlimited_resources(&self, resources: &mut M<Resource>) {
        let width = self.world.width();
        let height = self.world.height();

        for x in 0..width {
            for y in 0..height {
                self.add_unlimited_resource(resources, &v2(x, y));
            }
        }
    }

    fn add_unlimited_resource(&self, resources: &mut M<Resource>, position: &V2<usize>) {
        if *resources.get_cell_unsafe(position) != Resource::None {
            return;
        }
        RESOURCES
            .iter()
            .filter(|&resource| {
                count(*resource).is_none() && self.is_candidate(*resource, position)
            })
            .for_each(|resource| *resources.mut_cell_unsafe(position) = *resource);
    }

    pub fn load_resources(&mut self, resources: &M<Resource>) {
        for x in 0..resources.width() {
            for y in 0..resources.height() {
                let position = v2(x, y);
                self.world.mut_cell_unsafe(&position).resource =
                    *resources.get_cell_unsafe(&position);
            }
        }
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
            Resource::Wood => {
                self.has_vegetation(position)
                    && !self.has_vegetation_type(position, VegetationType::Cactus)
            }
            _ => false,
        }
    }

    fn has_vegetation(&self, position: &V2<usize>) -> bool {
        match self.world.get_cell(position) {
            Some(WorldCell {
                object: WorldObject::Vegetation(..),
                ..
            }) => true,
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

fn count(resource: Resource) -> Option<usize> {
    match resource {
        Resource::Bananas => Some(8),
        Resource::Coal => Some(8),
        Resource::Deer => Some(8),
        Resource::Fur => Some(8),
        Resource::Gems => Some(4),
        Resource::Gold => Some(2),
        Resource::Iron => Some(8),
        Resource::Ivory => Some(8),
        Resource::Spice => Some(8),
        _ => None,
    }
}
