use crate::world::World;
use commons::*;
use isometric::drawing::*;
use isometric::terrain::*;
use isometric::*;
use std::collections::HashSet;

#[derive(Hash, PartialEq, Eq, Debug)]
struct Slab {
    from: V2<usize>,
    slab_size: usize,
}

impl Slab {
    fn new(point: V2<usize>, slab_size: usize) -> Slab {
        let from = (point / slab_size) * slab_size;
        Slab { from, slab_size }
    }

    fn to(&self) -> V2<usize> {
        v2(self.from.x + self.slab_size, self.from.y + self.slab_size)
    }
}

pub struct WorldArtist {
    width: usize,
    height: usize,
    drawing: TerrainDrawing,
    colors: M<Color>,
    shading: Box<SquareColoring>,
    slab_size: usize,
}

impl WorldArtist {
    pub fn new(
        world: &World,
        slab_size: usize,
        beach_level: f32,
        snow_level: f32,
        cliff_gradient: f32,
        light_direction: V3<f32>,
    ) -> WorldArtist {
        let (width, height) = world.terrain().elevations().shape();
        WorldArtist {
            width,
            height,
            drawing: TerrainDrawing::new(width, height, slab_size),
            colors: WorldArtist::get_colors(world, beach_level, snow_level, cliff_gradient),
            shading: WorldArtist::get_shading(light_direction),
            slab_size,
        }
    }

    fn get_shading(light_direction: V3<f32>) -> Box<SquareColoring> {
        Box::new(AngleSquareColoring::new(
            Color::new(1.0, 1.0, 1.0, 1.0),
            light_direction,
        ))
    }

    fn get_colors(
        world: &World,
        beach_level: f32,
        snow_level: f32,
        cliff_gradient: f32,
    ) -> M<Color> {
        let (width, height) = world.terrain().elevations().shape();
        M::from_fn(width - 1, height - 1, |x, y| {
            WorldArtist::get_color(world, &v2(x, y), beach_level, snow_level, cliff_gradient)
        })
    }

    fn get_color(
        world: &World,
        position: &V2<usize>,
        beach_level: f32,
        snow_level: f32,
        cliff_gradient: f32,
    ) -> Color {
        let max_gradient = world.get_max_abs_rise(&position);
        let min_elevation = world.get_lowest_corner(&position);
        if min_elevation > snow_level {
            Color::new(1.0, 1.0, 1.0, 1.0)
        } else if max_gradient > cliff_gradient {
            Color::new(0.5, 0.4, 0.3, 1.0)
        } else if min_elevation < beach_level {
            Color::new(1.0, 1.0, 0.0, 1.0)
        } else {
            Color::new(0.0, 0.75, 0.0, 1.0)
        }
    }

    pub fn draw_terrain(&self) -> Command {
        Command::Draw {
            name: "terrain".to_string(),
            drawing: Box::new(self.drawing.clone()),
        }
    }

    pub fn draw_sea(&self, world: &World) -> Command {
        Command::Draw {
            name: "sea".to_string(),
            drawing: Box::new(SeaDrawing::new(
                self.width as f32,
                self.height as f32,
                world.sea_level(),
            )),
        }
    }

    fn draw_slab(&mut self, world: &World, slab: &Slab) -> Vec<Command> {
        self.draw_slab_tiles(world, slab);
        self.draw_slab_rivers_roads(world, &slab)
    }

    fn draw_slab_tiles(&mut self, world: &World, slab: &Slab) {
        let to = slab.to();
        let to = v2(to.x.min(self.width - 1), to.y.min(self.height - 1));
        self.drawing
            .update(world.terrain(), &self.colors, &self.shading, slab.from, to);
    }

    fn get_road_river_nodes(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> (Vec<Node>, Vec<Node>) {
        let mut road_nodes = vec![];
        let mut river_nodes = vec![];
        for x in from.x..to.x {
            for y in from.y..to.y {
                let road_node = world.roads().get_node(v2(x, y));
                let river_node = world.rivers().get_node(v2(x, y));
                if road_node.width() > 0.0 || road_node.height() > 0.0 {
                    road_nodes.push(road_node);
                } else if river_node.width() > 0.0 || river_node.height() > 0.0 {
                    river_nodes.push(river_node)
                }
            }
        }
        (road_nodes, river_nodes)
    }

    fn draw_slab_rivers_roads(&mut self, world: &World, slab: &Slab) -> Vec<Command> {
        let river_color = &Color::new(0.0, 0.0, 1.0, 1.0);
        let road_color = &Color::new(0.5, 0.5, 0.5, 1.0);
        let from = &slab.from;
        let to = &slab.to();
        let river_edges = world.rivers().get_edges(from, to);
        let road_edges = world.roads().get_edges(from, to);
        let (road_nodes, river_nodes) = self.get_road_river_nodes(world, from, to);
        vec![
            Command::Draw {
                name: format!("{:?}-river-edges", slab.from),
                drawing: Box::new(EdgeDrawing::new(
                    world.terrain(),
                    &river_edges,
                    &river_color,
                    0.0,
                )),
            },
            Command::Draw {
                name: format!("{:?}-road-edges", slab.from),
                drawing: Box::new(EdgeDrawing::new(
                    world.terrain(),
                    &road_edges,
                    &road_color,
                    0.0,
                )),
            },
            Command::Draw {
                name: format!("{:?}-river-nodes", slab.from),
                drawing: Box::new(NodeDrawing::new(
                    world.terrain(),
                    &river_nodes,
                    &river_color,
                    0.0,
                )),
            },
            Command::Draw {
                name: format!("{:?}-road-nodes", slab.from),
                drawing: Box::new(NodeDrawing::new(
                    world.terrain(),
                    &road_nodes,
                    &road_color,
                    0.0,
                )),
            },
        ]
    }

    fn draw_slabs(&mut self, world: &World, slabs: HashSet<Slab>) -> Vec<Command> {
        let mut out = vec![];
        for slab in slabs {
            out.append(&mut self.draw_slab(world, &slab));
        }
        out.push(self.draw_terrain());
        out
    }

    fn get_affected_slabs(&self, world: &World, positions: &Vec<V2<usize>>) -> HashSet<Slab> {
        positions
            .into_iter()
            .flat_map(|position| world.expand_position(&position))
            .map(|position| Slab::new(position, self.slab_size))
            .collect()
    }

    pub fn draw_affected(&mut self, world: &World, positions: &Vec<V2<usize>>) -> Vec<Command> {
        self.draw_slabs(world, self.get_affected_slabs(world, positions))
    }

    fn get_all_slabs(&self) -> HashSet<Slab> {
        let mut out = HashSet::new();
        for x in 0..self.width / self.slab_size {
            for y in 0..self.height / self.slab_size {
                let from = v2(x * self.slab_size, y * self.slab_size);
                out.insert(Slab::new(from, self.slab_size));
            }
        }
        out
    }

    fn draw_all(&mut self, world: &World) -> Vec<Command> {
        self.draw_slabs(world, self.get_all_slabs())
    }

    pub fn init(&mut self, world: &World) -> Vec<Command> {
        let mut out = vec![];
        out.push(self.draw_terrain());
        out.push(self.draw_sea(world));
        out.append(&mut self.draw_all(world));
        out
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn slab_new() {
        assert_eq!(
            Slab::new(v2(11, 33), 32),
            Slab {
                from: v2(0, 32),
                slab_size: 32,
            }
        );
    }

    #[test]
    fn slab_to() {
        assert_eq!(Slab::new(v2(11, 33), 32).to(), v2(32, 64));
    }

}
