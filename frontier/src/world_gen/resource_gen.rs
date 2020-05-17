use super::*;
use crate::world::*;
use commons::edge::Edge;
use commons::equalize::{equalize_with_filter, PositionValue};
use commons::perlin::stacked_perlin_noise;
use commons::rand::prelude::*;
use commons::rand::seq::SliceRandom;
use commons::*;
use std::collections::{BTreeMap, HashSet};
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
    shallow_depth_pc: f32,
    cliff_edges_for_cliff: (usize, usize),
}

impl Default for ResourceParams {
    fn default() -> ResourceParams {
        ResourceParams {
            farmland: FarmlandConstraints::default(),
            shallow_depth_pc: 0.25,
            cliff_edges_for_cliff: (1, 2),
        }
    }
}

pub struct ResourceGen<'a, R: Rng> {
    power: usize,
    world: &'a mut World,
    params: &'a WorldGenParameters,
    rng: &'a mut R,
}

impl<'a, R: Rng> ResourceGen<'a, R> {
    pub fn new(
        power: usize,
        world: &'a mut World,
        params: &'a WorldGenParameters,
        rng: &'a mut R,
    ) -> ResourceGen<'a, R> {
        ResourceGen {
            power,
            world,
            params,
            rng,
        }
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
            let count = match count(resource) {
                Some(count) if count > 0 => count,
                _ => continue,
            };
            candidates.retain(|candidate| !taken.contains(candidate));
            let candidates = self.reduce_candidates(candidates, resource, count);
            let chosen = candidates.choose_multiple(&mut self.rng, count);
            for choice in chosen {
                *resources.mut_cell_unsafe(choice) = resource;
                taken.insert(*choice);
            }
        }
    }

    fn get_candidates(&self) -> BTreeMap<Resource, Vec<V2<usize>>> {
        let width = self.world.width();
        let height = self.world.height();

        let mut out = BTreeMap::new();

        for x in 0..width {
            for y in 0..height {
                let position = v2(x, y);
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

    fn reduce_candidates(
        &mut self,
        mut candidates: Vec<V2<usize>>,
        resource: Resource,
        resource_count: usize,
    ) -> Vec<V2<usize>> {
        let noise = equalize_with_filter(
            stacked_perlin_noise(
                self.world.width(),
                self.world.height(),
                self.rng.gen(),
                (0..self.power).map(|_| 1.0).collect(),
            ),
            &|PositionValue { position, .. }| self.is_candidate(resource, position),
        );
        candidates.sort_by(|a, b| {
            noise
                .get_cell_unsafe(a)
                .partial_cmp(&noise.get_cell_unsafe(b))
                .unwrap()
        });
        candidates.truncate(resource_count * spread(resource));
        candidates
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
            Resource::Bananas => {
                !self.is_sea(position)
                    && self.has_vegetation_type_adjacent(position, VegetationType::PalmTree)
            }
            Resource::Coal => !self.is_sea(position) && self.is_cliff(position),
            Resource::Crabs => self.in_shallow_sea(position),
            Resource::Deer => {
                !self.is_sea(position)
                    && self.is_flat(position)
                    && self.among_vegetation_type(position, VegetationType::DeciduousTree)
            }
            Resource::Farmland => !self.is_sea(position) && self.is_farmland_candidate(position),
            Resource::Fur => {
                !self.is_sea(position)
                    && self.has_vegetation_type_adjacent(position, VegetationType::EvergreenTree)
            }
            Resource::Gems => !self.is_sea(position),
            Resource::Gold => !self.is_sea(position) && self.in_river(position),
            Resource::Iron => !self.is_sea(position) && self.is_cliff(position),
            Resource::Ivory => {
                !self.is_sea(position)
                    && self.is_flat(position)
                    && self.among_vegetation_type(position, VegetationType::PalmTree)
            }
            Resource::Spice => {
                !self.is_sea(position)
                    && self.has_vegetation_type_adjacent(position, VegetationType::PalmTree)
            }
            Resource::Stone => !self.is_sea(position) && self.is_cliff(position),
            Resource::Truffles => {
                !self.is_sea(position)
                    && self.has_vegetation_type_adjacent(position, VegetationType::DeciduousTree)
            }
            Resource::Whales => self.in_deep_sea(position),
            Resource::Wood => {
                !self.is_sea(position)
                    && (self.has_vegetation_type_adjacent(position, VegetationType::PalmTree)
                        || self
                            .has_vegetation_type_adjacent(position, VegetationType::DeciduousTree)
                        || self
                            .has_vegetation_type_adjacent(position, VegetationType::EvergreenTree))
            }
            Resource::None => false,
        }
    }

    fn is_beach(&self, position: &V2<usize>) -> bool {
        let elevation = match self.world.get_cell(position) {
            Some(WorldCell { elevation, .. }) => elevation,
            None => return false,
        };
        *elevation > self.world.sea_level() && *elevation <= self.params.beach_level
    }

    fn is_sea(&self, position: &V2<usize>) -> bool {
        self.world.is_sea(position)
    }

    fn has_vegetation_type_adjacent(
        &self,
        position: &V2<usize>,
        vegetation_type: VegetationType,
    ) -> bool {
        self.world
            .get_adjacent_tiles_in_bounds(position)
            .iter()
            .any(|position| self.has_vegetation_type_on_tile(position, vegetation_type))
    }

    fn has_vegetation_type_on_tile(
        &self,
        position: &V2<usize>,
        vegetation_type: VegetationType,
    ) -> bool {
        match self.world.get_cell(position) {
            Some(WorldCell {
                object: WorldObject::Vegetation(actual),
                ..
            }) if *actual == vegetation_type => true,
            _ => false,
        }
    }

    fn among_vegetation_type(&self, position: &V2<usize>, vegetation_type: VegetationType) -> bool {
        if self
            .world
            .get_adjacent_tiles_in_bounds(position)
            .iter()
            .any(|tile| self.has_object(tile))
        {
            return false;
        }
        match self.world.get_cell(&position) {
            Some(WorldCell { climate, .. })
                if vegetation_type.in_range_temperature(climate.temperature)
                    && vegetation_type.in_range_groundwater(climate.groundwater) =>
            {
                true
            }
            _ => false,
        }
    }

    fn has_object(&self, position: &V2<usize>) -> bool {
        if let Some(WorldCell {
            object: WorldObject::None,
            ..
        }) = self.world.get_cell(position)
        {
            false
        } else {
            true
        }
    }

    fn is_cliff(&self, position: &V2<usize>) -> bool {
        let adjacent_cliff_edges = self.count_adjacent_cliff_edges(position);
        adjacent_cliff_edges >= self.params.resources.cliff_edges_for_cliff.0
            && adjacent_cliff_edges <= self.params.resources.cliff_edges_for_cliff.1
    }

    fn is_flat(&self, position: &V2<usize>) -> bool {
        self.count_adjacent_cliff_edges(position) == 0
    }

    fn count_adjacent_cliff_edges(&self, position: &V2<usize>) -> usize {
        self.get_adjacent_edges(position)
            .iter()
            .flat_map(|edge| self.world.get_rise(edge.from(), edge.to()))
            .filter(|rise| rise.abs() > self.params.cliff_gradient)
            .count()
    }

    fn get_adjacent_edges(&self, position: &V2<usize>) -> Vec<Edge> {
        let x = position.x;
        let y = position.y;
        let mut edges = vec![
            Edge::new(v2(x, y), v2(x + 1, y)),
            Edge::new(v2(x, y), v2(x, y + 1)),
        ];
        if x > 0 {
            edges.push(Edge::new(v2(x, y), v2(x - 1, y)));
        }
        if y > 0 {
            edges.push(Edge::new(v2(x, y), v2(x, y - 1)));
        }
        edges
    }

    fn in_shallow_sea(&self, position: &V2<usize>) -> bool {
        self.in_sea_between_depths(position, 0.0, self.params.resources.shallow_depth_pc)
    }

    fn in_deep_sea(&self, position: &V2<usize>) -> bool {
        self.in_sea_between_depths(position, self.params.resources.shallow_depth_pc, 1.0)
    }

    fn in_sea_between_depths(
        &self,
        position: &V2<usize>,
        from_depth_pc: f32,
        to_depth_pc: f32,
    ) -> bool {
        let from_depth = self.world.sea_level() * (1.0 - from_depth_pc);
        let to_depth = self.world.sea_level() * (1.0 - to_depth_pc);
        if let Some(WorldCell { elevation, .. }) = self.world.get_cell(position) {
            *elevation >= to_depth && *elevation <= from_depth
        } else {
            false
        }
    }

    fn is_farmland_candidate(&self, position: &V2<usize>) -> bool {
        if self.is_sea(position)
            || self.has_object(position)
            || self.world.get_max_abs_rise(position) > self.params.resources.farmland.max_slope
        {
            return false;
        }
        match self.world.get_cell(position) {
            Some(WorldCell { climate, .. })
                if climate.temperature >= self.params.resources.farmland.min_temperature
                    && climate.groundwater >= self.params.resources.farmland.min_groundwater =>
            {
                true
            }
            _ => false,
        }
    }

    fn in_river(&self, position: &V2<usize>) -> bool {
        let cell = match self.world.get_cell(position) {
            Some(cell) => cell,
            None => return false,
        };
        cell.river.here()
    }
}

fn count(resource: Resource) -> Option<usize> {
    match resource {
        Resource::Bananas => Some(8),
        Resource::Coal => Some(8),
        Resource::Crabs => Some(8),
        Resource::Deer => Some(8),
        Resource::Fur => Some(8),
        Resource::Gems => Some(4),
        Resource::Gold => Some(2),
        Resource::Iron => Some(8),
        Resource::Ivory => Some(6),
        Resource::Spice => Some(8),
        Resource::Truffles => Some(6),
        Resource::Whales => Some(8),
        _ => None,
    }
}

fn spread(resource: Resource) -> usize {
    match resource {
        Resource::Bananas => 32,
        Resource::Coal => 8,
        Resource::Crabs => 32,
        Resource::Deer => 32,
        Resource::Fur => 32,
        Resource::Gems => 8,
        Resource::Gold => 8,
        Resource::Iron => 8,
        Resource::Ivory => 32,
        Resource::Spice => 8,
        Resource::Truffles => 32,
        Resource::Whales => 128,
        _ => 1,
    }
}
