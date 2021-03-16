use crate::world::*;
use commons::barycentric::triangle_interpolate_any;
use commons::grid::Grid;
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
pub struct VegatationArtistParams {
    exaggeration: f32,
}

impl Default for VegatationArtistParams {
    fn default() -> VegatationArtistParams {
        VegatationArtistParams {
            exaggeration: 100.0,
        }
    }
}

#[derive(Clone)]
pub struct VegetationArtist {
    params: VegatationArtistParams,
}

impl VegetationArtist {
    pub fn new(params: VegatationArtistParams) -> VegetationArtist {
        VegetationArtist { params }
    }

    pub fn draw(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Vec<Command> {
        let mut vegetation = HashMap::new();

        for x in from.x..to.x {
            for y in from.y..to.y {
                let position = v2(x, y);

                let cell = unwrap_or!(world.get_cell(&position), continue);
                if !cell.is_visible() {
                    continue;
                }

                if let WorldObject::Vegetation {
                    vegetation_type,
                    offset,
                } = cell.object
                {
                    snap_to_terrain(&world, &position, offset)
                        .or_else(|| snap_to_middle(world, &position))
                        .map(|WorldCoord { x, y, z }| {
                            WorldCoord::new(
                                x,
                                y,
                                z + (vegetation_type.height() * self.params.exaggeration) / 2.0,
                            )
                        })
                        .into_iter()
                        .for_each(|world_coord| {
                            vegetation
                                .entry(vegetation_type)
                                .or_insert_with(Vec::new)
                                .push(world_coord)
                        });
                }
            }
        }

        self.create_billboards(from, vegetation)
    }

    fn create_billboards<T: Debug>(
        &self,
        label: T,
        vegetation: HashMap<VegetationType, Vec<WorldCoord>>,
    ) -> Vec<Command> {
        let mut out = vec![];

        let texture_coords = &Rectangle::Unit();

        for (vegetation_type, world_coords) in vegetation {
            let size = vegetation_type.height() * self.params.exaggeration;
            out.append(&mut create_and_update_billboards(
                format!("{:?}-{:?}", label, vegetation_type.name()),
                texture(vegetation_type),
                world_coords
                    .iter()
                    .map(|world_coord| Billboard {
                        world_coord,
                        width: &size,
                        height: &size,
                        texture_coords,
                    })
                    .collect(),
            ));
        }

        out
    }
}

fn snap_to_terrain(world: &World, tile: &V2<usize>, offset: V2<f32>) -> Option<WorldCoord> {
    let geometry = TerrainGeometry::of(world);
    let triangles = geometry.get_triangles_for_tile(&tile);
    let position = v2(tile.x as f32 + offset.x, tile.y as f32 + offset.y);
    triangle_interpolate_any(&position, &triangles)
        .map(|z| WorldCoord::new(position.x, position.y, z))
}

fn snap_to_middle(world: &World, tile: &V2<usize>) -> Option<WorldCoord> {
    let position = v2(tile.x as f32 + 0.5, tile.y as f32 + 0.5);
    world
        .snap_to_middle(&position)
        .map(|z| WorldCoord::new(position.x, position.y, z))
}

fn texture(vegetation_type: VegetationType) -> &'static str {
    match vegetation_type {
        VegetationType::SnowTree => "resources/textures/twemoji/derivative/snow_pine.png",
        VegetationType::EvergreenTree => "resources/textures/twemoji/derivative/pine.png",
        VegetationType::DeciduousTree => "resources/textures/fxemoji/tree.png",
        VegetationType::PalmTree => "resources/textures/fxemoji/palm.png",
        VegetationType::Cactus => "resources/textures/fxemoji/cactus.png",
    }
}
