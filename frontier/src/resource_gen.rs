use super::*;
use crate::resource::{Resource, Resources, RESOURCES};
use crate::world::*;
use commons::edge::Edge;
use commons::equalize::{equalize_with_filter, PositionValue};
use commons::grid::Grid;
use commons::perlin::stacked_perlin_noise;
use commons::rand::prelude::*;
use commons::rand::seq::SliceRandom;
use commons::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::default::Default;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FarmlandConstraints {
    pub min_groundwater: f32,
    pub max_crops_slope: f32,
    pub min_temperature: f32,
}

impl Default for FarmlandConstraints {
    fn default() -> FarmlandConstraints {
        FarmlandConstraints {
            min_groundwater: 0.2,
            max_crops_slope: 0.2,
            min_temperature: 0.0,
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ResourceGenParameters {
    farmland: FarmlandConstraints,
    shallow_depth_pc: f32,
    cliff_edges_for_cliff: (usize, usize),
}

impl Default for ResourceGenParameters {
    fn default() -> ResourceGenParameters {
        ResourceGenParameters {
            farmland: FarmlandConstraints::default(),
            shallow_depth_pc: 0.25,
            cliff_edges_for_cliff: (1, 2),
        }
    }
}

pub struct ResourceGen<'a, R: Rng> {
    params: &'a Parameters,
    world: &'a World,
    rng: &'a mut R,
}

impl<'a, R: Rng> ResourceGen<'a, R> {
    pub fn new(params: &'a Parameters, world: &'a World, rng: &'a mut R) -> ResourceGen<'a, R> {
        ResourceGen { params, world, rng }
    }

    pub fn compute_resources(&mut self) -> Resources {
        let width = self.world.width();
        let height = self.world.height();
        let mut out = Resources::new(width, height, HashSet::with_capacity(0));
        self.add_limited_resources(&mut out);
        self.add_unlimited_resources(&mut out);
        out
    }

    fn add_limited_resources(&mut self, resources: &mut Resources) {
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
                *resources.mut_cell_unsafe(choice) = hashset! {resource};
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
                        out.entry(*resource).or_insert_with(Vec::new).push(position)
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
                (0..self.params.power).map(|_| 1.0).collect(),
            ),
            &|PositionValue { position, .. }| self.is_candidate(resource, position),
        );
        candidates.sort_by(|a, b| {
            noise
                .get_cell_unsafe(a)
                .partial_cmp(noise.get_cell_unsafe(b))
                .unwrap()
        });
        candidates.truncate(resource_count * spread(resource));
        candidates
    }

    fn add_unlimited_resources(&self, resources: &mut Resources) {
        let width = self.world.width();
        let height = self.world.height();

        for x in 0..width {
            for y in 0..height {
                self.add_unlimited_resource(resources, &v2(x, y));
            }
        }
    }

    fn add_unlimited_resource(&self, resources: &mut Resources, position: &V2<usize>) {
        [
            Resource::Crops,
            Resource::Pasture,
            Resource::Shelter,
            Resource::Stone,
            Resource::Wood,
        ]
        .iter()
        .filter(|&resource| self.is_candidate(*resource, position))
        .for_each(|resource| {
            resources.mut_cell_unsafe(position).insert(*resource);
        });
    }

    fn is_candidate(&self, resource: Resource, position: &V2<usize>) -> bool {
        if let Resource::Shelter = resource {
            return !self.is_sea(position) && !self.tile_is_cliff(position);
        }
        if self.is_beach(position) {
            return false;
        }
        match resource {
            Resource::Bananas => {
                !self.is_sea(position)
                    && self.has_vegetation_type_adjacent(position, VegetationType::PalmTree)
            }
            Resource::Bison => {
                !self.is_sea(position)
                    && self.is_flat(position)
                    && self.among_vegetation_type(position, VegetationType::EvergreenTree)
            }
            Resource::Coal => !self.is_sea(position) && self.is_accessible_cliff(position),
            Resource::Crabs => self.in_shallow_sea(position),
            Resource::Crops => {
                !self.tile_is_beach(position)
                    && self.tile_by_river(position)
                    && self.tile_is_arable_gradient(position)
                    && self.tile_is_farmable_climate(position)
            }
            Resource::Deer => {
                !self.is_sea(position)
                    && self.is_flat(position)
                    && self.among_vegetation_type(position, VegetationType::DeciduousTree)
            }
            Resource::Fur => {
                !self.is_sea(position)
                    && (self.has_vegetation_type_adjacent(position, VegetationType::EvergreenTree)
                        || self.has_vegetation_type_adjacent(position, VegetationType::SnowTree))
            }
            Resource::Gems => !self.is_sea(position),
            Resource::Gold => !self.is_sea(position) && self.in_river(position),
            Resource::Iron => !self.is_sea(position) && self.is_accessible_cliff(position),
            Resource::Ivory => {
                !self.is_sea(position)
                    && self.is_flat(position)
                    && self.among_vegetation_type(position, VegetationType::PalmTree)
            }
            Resource::Pasture => {
                !self.tile_is_beach(position)
                    && !self.tile_is_cliff(position)
                    && self.tile_is_farmable_climate(position)
            }
            Resource::Spice => {
                !self.is_sea(position)
                    && self.has_vegetation_type_adjacent(position, VegetationType::PalmTree)
            }
            Resource::Stone => !self.is_sea(position) && self.is_accessible_cliff(position),
            Resource::Truffles => {
                !self.is_sea(position)
                    && self.has_vegetation_type_adjacent(position, VegetationType::DeciduousTree)
            }
            Resource::Whales => self.in_deep_sea(position),
            Resource::Wood => {
                !self.is_sea(position)
                    && (self.tile_has_vegetation_type(position, VegetationType::PalmTree)
                        || self.tile_has_vegetation_type(position, VegetationType::DeciduousTree)
                        || self.tile_has_vegetation_type(position, VegetationType::EvergreenTree)
                        || self.tile_has_vegetation_type(position, VegetationType::SnowTree))
            }
            _ => false,
        }
    }

    fn is_beach(&self, position: &V2<usize>) -> bool {
        let elevation = match self.world.get_cell(position) {
            Some(WorldCell { elevation, .. }) => elevation,
            None => return false,
        };
        *elevation > self.world.sea_level() && *elevation <= self.params.world_gen.beach_level
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
            .any(|position| self.tile_has_vegetation_type(position, vegetation_type))
    }

    fn among_vegetation_type(&self, position: &V2<usize>, vegetation_type: VegetationType) -> bool {
        if self
            .world
            .get_adjacent_tiles_in_bounds(position)
            .iter()
            .any(|tile| self.tile_has_object(tile))
        {
            return false;
        }
        let temperature = unwrap_or!(self.world.tile_avg_temperature(position), return false);
        let groundwater = unwrap_or!(self.world.tile_avg_groundwater(position), return false);
        vegetation_type.in_range_temperature(temperature)
            && vegetation_type.in_range_groundwater(groundwater)
    }

    fn is_flat(&self, position: &V2<usize>) -> bool {
        self.count_adjacent_cliff_edges(position) == 0
    }

    fn count_adjacent_cliff_edges(&self, position: &V2<usize>) -> usize {
        self.get_adjacent_edges(position)
            .iter()
            .flat_map(|edge| self.world.get_rise(edge.from(), edge.to()))
            .filter(|rise| rise.abs() >= self.params.world_gen.cliff_gradient)
            .count()
    }

    fn is_accessible_cliff(&self, position: &V2<usize>) -> bool {
        let adjacent_tiles_in_bounds = self.world.get_adjacent_tiles_in_bounds(position);

        let adjacent_cliffs = adjacent_tiles_in_bounds
            .iter()
            .filter(|tile| {
                self.world.get_max_abs_rise(tile) >= self.params.world_gen.cliff_gradient
            })
            .count();

        let adjacent_not_cliffs = adjacent_tiles_in_bounds.len() - adjacent_cliffs;

        adjacent_cliffs > 0 && adjacent_not_cliffs > 0
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
        self.in_sea_between_depths(position, 0.0, self.params.resource_gen.shallow_depth_pc)
    }

    fn in_deep_sea(&self, position: &V2<usize>) -> bool {
        self.in_sea_between_depths(position, self.params.resource_gen.shallow_depth_pc, 1.0)
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

    fn in_river(&self, position: &V2<usize>) -> bool {
        let cell = unwrap_or!(self.world.get_cell(position), return false);
        cell.river.here()
    }

    fn tile_has_vegetation_type(
        &self,
        position: &V2<usize>,
        vegetation_type: VegetationType,
    ) -> bool {
        matches!(
            self.world.get_cell(position),
            Some(WorldCell {
                object:
                    WorldObject::Vegetation {
                        vegetation_type: actual,
                        ..
                    },
                ..
            }) if *actual == vegetation_type
        )
    }

    fn tile_has_object(&self, position: &V2<usize>) -> bool {
        !matches!(
            self.world.get_cell(position),
            Some(WorldCell {
                object: WorldObject::None,
                ..
            })
        )
    }

    fn tile_is_beach(&self, position: &V2<usize>) -> bool {
        self.world.get_lowest_corner(position) <= self.params.world_gen.beach_level
    }

    fn tile_is_farmable_climate(&self, position: &V2<usize>) -> bool {
        let temperature = unwrap_or!(self.world.tile_avg_temperature(position), return false);
        let groundwater = unwrap_or!(self.world.tile_avg_groundwater(position), return false);
        temperature >= self.params.resource_gen.farmland.min_temperature
            && groundwater >= self.params.resource_gen.farmland.min_groundwater
    }

    fn tile_is_cliff(&self, position: &V2<usize>) -> bool {
        self.world.get_max_abs_rise(position) >= self.params.world_gen.cliff_gradient
    }

    fn tile_is_arable_gradient(&self, position: &V2<usize>) -> bool {
        self.world.get_max_abs_rise(position) <= self.params.resource_gen.farmland.max_crops_slope
    }

    fn tile_by_river(&self, position: &V2<usize>) -> bool {
        self.world
            .get_corners_in_bounds(position)
            .iter()
            .any(|corner| self.world.get_cell_unsafe(corner).river.here())
    }
}

fn count(resource: Resource) -> Option<usize> {
    match resource {
        Resource::Bananas => Some(16),
        Resource::Bison => Some(16),
        Resource::Coal => Some(16),
        Resource::Crabs => Some(16),
        Resource::Deer => Some(16),
        Resource::Fur => Some(16),
        Resource::Gems => Some(4),
        Resource::Gold => Some(2),
        Resource::Iron => Some(16),
        Resource::Ivory => Some(16),
        Resource::Spice => Some(16),
        Resource::Truffles => Some(16),
        Resource::Whales => Some(16),
        _ => None,
    }
}

fn spread(resource: Resource) -> usize {
    match resource {
        Resource::Bananas => 64,
        Resource::Bison => 64,
        Resource::Coal => 16,
        Resource::Crabs => 64,
        Resource::Deer => 64,
        Resource::Fur => 64,
        Resource::Gems => 8,
        Resource::Gold => 8,
        Resource::Iron => 16,
        Resource::Ivory => 64,
        Resource::Spice => 64,
        Resource::Truffles => 64,
        Resource::Whales => 128,
        _ => 1,
    }
}
