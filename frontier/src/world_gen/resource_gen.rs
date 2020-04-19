use super::*;
use crate::world::*;
use commons::rand::prelude::*;
use commons::*;
use std::default::Default;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct RandomResource {
    resource: Resource,
    probability: f32,
}

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
    random_resources: Vec<RandomResource>,
    farmland: FarmlandConstraints,
}

impl Default for ResourceParams {
    fn default() -> ResourceParams {
        ResourceParams {
            random_resources: vec![
                RandomResource {
                    resource: Resource::Gems,
                    probability: 1.0 / 16384.0,
                },
                RandomResource {
                    resource: Resource::Oranges,
                    probability: 1.0 / 4096.0,
                },
            ],
            farmland: FarmlandConstraints::default(),
        }
    }
}

pub fn compute_resources<R: Rng>(
    world: &mut World,
    params: &WorldGenParameters,
    rng: &mut R,
) -> M<Resource> {
    let width = world.width() - 1;
    let height = world.height() - 1;
    let mut out = M::from_element(width, height, Resource::None);

    for x in 0..width {
        for y in 0..height {
            let position = v2(x, y);
            if world.is_sea(&position) {
                continue;
            }
            if let Some(resource) = get_random_resource(rng, &params.resources.random_resources) {
                out[(x, y)] = resource;
            } else if is_farmland_candidate(world, &params, &position) {
                out[(x, y)] = Resource::Farmland;
            }
        }
    }
    out
}

pub fn load_resources(world: &mut World, resources: &M<Resource>) {
    for x in 0..resources.width() {
        for y in 0..resources.height() {
            world.mut_cell_unsafe(&v2(x, y)).resource = resources[(x, y)];
        }
    }
}

fn get_random_resource<R: Rng>(
    rng: &mut R,
    random_resources: &[RandomResource],
) -> Option<Resource> {
    let r = rng.gen_range(0.0, 1.0);
    let mut cum = 0.0;
    for resource in random_resources {
        cum += resource.probability;
        if r <= cum {
            return Some(resource.resource);
        }
    }
    None
}

fn is_farmland_candidate(world: &World, params: &WorldGenParameters, position: &V2<usize>) -> bool {
    let constraints = &params.resources.farmland;
    let beach_level = params.beach_level;
    if position.x == world.width() - 1 || position.y == world.height() - 1 {
        return false;
    };
    match world.tile_avg_temperature(&position) {
        Some(temperature) if temperature >= constraints.min_temperature => (),
        _ => return false,
    };
    match world.tile_avg_groundwater(&position) {
        Some(groundwater) if groundwater >= constraints.min_groundwater => (),
        _ => return false,
    };
    world
        .get_cell(position)
        .map(|cell| {
            cell.object == WorldObject::None
                && world.get_max_abs_rise(position) <= constraints.max_slope
                && world.get_lowest_corner(position) > beach_level
        })
        .unwrap_or(false)
}
