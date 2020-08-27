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
                let mut world_coord =
                    match world.snap_to_middle(WorldCoord::new(x as f32, y as f32, 0.0)) {
                        Some(world_coord) => world_coord,
                        None => continue,
                    };
                let cell = unwrap_or!(world.get_cell(&position), continue);
                if !cell.is_visible() {
                    continue;
                }

                if let WorldObject::Vegetation {
                    vegetation_type, ..
                } = cell.object
                {
                    world_coord.z += (vegetation_type.height() * self.params.exaggeration) / 2.0;

                    vegetation
                        .entry(vegetation_type)
                        .or_insert_with(Vec::new)
                        .push(world_coord);
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

        for (vegetation_type, billboards) in vegetation {
            let size = vegetation_type.height() * self.params.exaggeration;
            out.append(&mut create_and_update_billboards(
                format!("{:?}-{:?}", label, vegetation_type.name()),
                billboards,
                size,
                size,
                texture(vegetation_type),
            ));
        }

        out
    }
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
