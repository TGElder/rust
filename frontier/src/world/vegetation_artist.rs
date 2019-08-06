use crate::world::*;
use commons::*;
use isometric::cell_traits::*;
use isometric::coords::*;
use isometric::drawing::*;
use isometric::Command;

pub struct VegetationArtist {}

impl VegetationArtist {
    pub fn new() -> VegetationArtist {
        VegetationArtist {}
    }

    pub fn draw(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Vec<Command> {
        let mut trees = vec![];
        let mut palms = vec![];
        let mut pines = vec![];
        let mut cacti = vec![];
        for x in from.x..to.x {
            for y in from.y..to.y {
                let position = v2(x, y);
                if let Some(mut world_coord) =
                    world.snap_to_middle(WorldCoord::new(x as f32, y as f32, 0.0))
                {
                    let cell = world.get_cell_unsafe(&position);

                    if !cell.is_visible() || cell.climate.vegetation == Vegetation::None {
                        continue;
                    }

                    world_coord.z += cell.climate.vegetation.height();

                    match cell.climate.vegetation {
                        Vegetation::PalmTree => palms.push(world_coord),
                        Vegetation::DeciduousTree => trees.push(world_coord),
                        Vegetation::EvergreenTree => pines.push(world_coord),
                        Vegetation::Cactus => cacti.push(world_coord),
                        Vegetation::None => (),
                    };
                }
            }
        }
        let mut out = vec![];

        out.append(&mut draw_billboards(
            format!("{:?}-trees", from).to_string(),
            trees,
            Vegetation::DeciduousTree.height(),
            Vegetation::DeciduousTree.height(),
            "tree.png",
        ));
        out.append(&mut draw_billboards(
            format!("{:?}-palms", from).to_string(),
            palms,
            Vegetation::PalmTree.height(),
            Vegetation::PalmTree.height(),
            "palm.png",
        ));
        out.append(&mut draw_billboards(
            format!("{:?}-pines", from).to_string(),
            pines,
            Vegetation::EvergreenTree.height(),
            Vegetation::EvergreenTree.height(),
            "pine.png",
        ));
        out.append(&mut draw_billboards(
            format!("{:?}-cacti", from).to_string(),
            cacti,
            Vegetation::Cactus.height(),
            Vegetation::Cactus.height(),
            "cactus.png",
        ));
        out
    }
}
