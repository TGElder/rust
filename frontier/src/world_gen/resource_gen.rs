use super::*;
use crate::world::*;
use commons::rand::prelude::*;
use commons::*;
use std::default::Default;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ResourceParams {
    gem_probability: f32,
}

impl Default for ResourceParams {
    fn default() -> ResourceParams {
        ResourceParams {
            gem_probability: 1.0 / 1000.0,
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
            let r = rng.gen_range(0.0, 1.0);
            if r <= params.resources.gem_probability {
                out[(x, y)] = Resource::Gems
            };
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
