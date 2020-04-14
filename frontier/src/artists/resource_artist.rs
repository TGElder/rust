use crate::world::*;
use commons::*;
use isometric::cell_traits::*;
use isometric::coords::*;
use isometric::drawing::*;
use isometric::Command;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Debug;

#[derive(Clone, Copy)]
pub struct ResourceArtistParameters {
    size: f32,
    hover: f32,
}

impl Default for ResourceArtistParameters {
    fn default() -> ResourceArtistParameters {
        ResourceArtistParameters {
            size: 0.75,
            hover: 0.25,
        }
    }
}

pub struct ResourceArtist {
    params: ResourceArtistParameters,
}

impl ResourceArtist {
    pub fn new(params: ResourceArtistParameters) -> ResourceArtist {
        ResourceArtist { params }
    }

    pub fn draw(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Vec<Command> {
        let mut resources = HashMap::new();
        for x in from.x..to.x {
            for y in from.y..to.y {
                let position = v2(x, y);
                let mut world_coord =
                    match world.snap_to_middle(WorldCoord::new(x as f32, y as f32, 0.0)) {
                        Some(world_coord) => world_coord,
                        None => continue,
                    };
                let cell = match world.get_cell(&position) {
                    Some(cell) => cell,
                    None => continue,
                };
                if !cell.is_visible() {
                    continue;
                }

                if texture(cell.resource).is_some() {
                    world_coord.z += self.params.hover + self.params.size / 2.0;
                    resources
                        .entry(cell.resource)
                        .or_insert_with(Vec::new)
                        .push(world_coord);
                }
            }
        }

        self.create_billboards(from, resources)
    }

    fn create_billboards<T: Debug>(
        &self,
        label: T,
        resources: HashMap<Resource, Vec<WorldCoord>>,
    ) -> Vec<Command> {
        let mut out = vec![];

        for (resource, billboards) in resources {
            out.append(&mut create_and_update_billboards(
                format!("{:?}-{:?}", label, resource.name()),
                billboards,
                self.params.size,
                self.params.size,
                texture(resource).unwrap(),
            ));
        }

        out
    }
}

fn texture(resource: Resource) -> Option<&'static str> {
    match resource {
        Resource::None => None,
        Resource::Gems => Some("gems.png"),
    }
}
