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

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ResourceParams {
    random_resources: Vec<RandomResource>,
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
            if world.is_sea(&v2(x, y)) {
                continue;
            }
            if let Some(resource) = get_random_resource(rng, &params.resources.random_resources) {
                out[(x, y)] = resource;
            }
        }
    }
    out
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

pub fn load_resources(world: &mut World, resources: &M<Resource>) {
    for x in 0..resources.width() {
        for y in 0..resources.height() {
            world.mut_cell_unsafe(&v2(x, y)).resource = resources[(x, y)];
        }
    }
}
