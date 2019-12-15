use crate::world::*;
use commons::*;
use isometric::cell_traits::*;
use isometric::coords::*;
use isometric::drawing::*;
use isometric::Command;

pub struct VegetationArtist {
    exaggeration: f32,
}

impl VegetationArtist {
    pub fn new(exaggeration: f32) -> VegetationArtist {
        VegetationArtist { exaggeration }
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

                    if !cell.is_visible() {
                        continue;
                    }

                    if let WorldObject::Vegetation(vegetation) = cell.object {
                        world_coord.z += (vegetation.height() * self.exaggeration) / 2.0;

                        match vegetation {
                            VegetationType::PalmTree => palms.push(world_coord),
                            VegetationType::DeciduousTree => trees.push(world_coord),
                            VegetationType::EvergreenTree => pines.push(world_coord),
                            VegetationType::Cactus => cacti.push(world_coord),
                        };
                    }
                }
            }
        }
        let mut out = vec![];

        let size = VegetationType::DeciduousTree.height() * self.exaggeration;
        out.append(&mut create_and_update_billboards(
            format!("{:?}-trees", from).to_string(),
            trees,
            size,
            size,
            "tree.png",
        ));

        let size = VegetationType::PalmTree.height() * self.exaggeration;
        out.append(&mut create_and_update_billboards(
            format!("{:?}-palms", from).to_string(),
            palms,
            size,
            size,
            "palm.png",
        ));

        let size = VegetationType::EvergreenTree.height() * self.exaggeration;
        out.append(&mut create_and_update_billboards(
            format!("{:?}-pines", from).to_string(),
            pines,
            size,
            size,
            "pine.png",
        ));

        let size = VegetationType::Cactus.height() * self.exaggeration;
        out.append(&mut create_and_update_billboards(
            format!("{:?}-cacti", from).to_string(),
            cacti,
            size,
            size,
            "cactus.png",
        ));
        out
    }
}
