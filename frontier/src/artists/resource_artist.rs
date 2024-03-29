use crate::resource::{Resource, Resources};
use crate::world::*;
use commons::grid::Grid;
use commons::index2d::Vec2D;
use commons::rectangle::Rectangle;
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
            size: 360.0 / 512.0,
            hover: 256.0 / 512.0,
        }
    }
}

#[derive(Clone)]
pub struct ResourceArtist {
    params: ResourceArtistParameters,
    draw_resources: Vec2D<Option<Resource>>,
}

impl ResourceArtist {
    pub fn new(params: ResourceArtistParameters, resources: &Resources) -> ResourceArtist {
        ResourceArtist {
            params,
            draw_resources: Self::compute_draw_resources(resources),
        }
    }

    pub fn compute_draw_resources(resources: &Resources) -> Vec2D<Option<Resource>> {
        let mut out = Vec2D::same_size_as(resources, None);
        for x in 0..resources.width() {
            for y in 0..resources.height() {
                let position = v2(x, y);
                for resource in resources.get_cell_unsafe(&position) {
                    if texture(*resource).is_some() {
                        out.set(&position, Some(*resource)).unwrap();
                    }
                }
            }
        }
        out
    }

    pub fn draw(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Vec<Command> {
        let mut resources = HashMap::new();
        for x in from.x..to.x {
            for y in from.y..to.y {
                let position = v2(x, y);
                let cell = unwrap_or!(world.get_cell(&position), continue);
                if !cell.is_visible() {
                    continue;
                }
                if let Ok(Some(resource)) = self.draw_resources.get(&position) {
                    let mut world_coord =
                        WorldCoord::new(x as f32, y as f32, cell.elevation.max(world.sea_level()));
                    world_coord.z += self.params.hover;
                    world_coord.z += self.params.size / 2.0;
                    resources
                        .entry(*resource)
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

        let texture_coords = &Rectangle::unit();

        for (resource, world_coords) in resources {
            out.append(&mut create_and_update_billboards(
                format!("{:?}-{:?}", label, resource.name()),
                texture(resource).unwrap(),
                world_coords
                    .iter()
                    .map(|world_coord| Billboard {
                        world_coord,
                        width: &self.params.size,
                        height: &self.params.size,
                        texture_coords,
                    })
                    .collect(),
            ));
        }

        out
    }
}

fn texture(resource: Resource) -> Option<&'static str> {
    match resource {
        Resource::Bananas => Some("resources/textures/twemoji/bananas.png"),
        Resource::Bison => Some("resources/textures/twemoji/bison.png"),
        Resource::Coal => Some("resources/textures/twemoji/derivative/coal.png"),
        Resource::Crabs => Some("resources/textures/twemoji/crabs.png"),
        Resource::Deer => Some("resources/textures/twemoji/deer.png"),
        Resource::Fur => Some("resources/textures/twemoji/fur.png"),
        Resource::Gems => Some("resources/textures/twemoji/gems.png"),
        Resource::Gold => Some("resources/textures/twemoji/gold.png"),
        Resource::Iron => Some("resources/textures/twemoji/derivative/iron.png"),
        Resource::Ivory => Some("resources/textures/twemoji/ivory.png"),
        Resource::Spice => Some("resources/textures/twemoji/spice.png"),
        Resource::Truffles => Some("resources/textures/twemoji/truffles.png"),
        Resource::Whales => Some("resources/textures/twemoji/whales.png"),
        _ => None,
    }
}
