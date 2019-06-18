use crate::world::*;
use commons::*;
use isometric::drawing::*;
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
    coloring: LayerColoring<WorldCell>,
    slab_size: usize,
}

impl WorldArtist {
    pub fn new(world: &World, coloring: LayerColoring<WorldCell>, slab_size: usize) -> WorldArtist {
        let width = world.width();
        let height = world.height();
        WorldArtist {
            width,
            height,
            drawing: TerrainDrawing::new("terrain".to_string(), width, height, slab_size),
            slab_size,
            coloring,
        }
    }

    pub fn coloring(&mut self) -> &mut LayerColoring<WorldCell> {
        &mut self.coloring
    }

    pub fn draw_terrain(&self) -> Vec<Command> {
        self.drawing.init()
    }

    fn draw_slab(&mut self, world: &World, slab: &Slab) -> Vec<Command> {
        let mut out = self.draw_slab_tiles(world, slab);
        out.append(&mut self.draw_slab_rivers_roads(world, &slab));
        out
    }

    fn draw_slab_tiles(&mut self, world: &World, slab: &Slab) -> Vec<Command> {
        let to = slab.to();
        let to = v2(to.x.min(self.width - 1), to.y.min(self.height - 1));
        self.drawing
            .update(world, world.sea_level(), &self.coloring, slab.from, to)
    }

    fn get_road_river_positions(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> (Vec<V2<usize>>, Vec<V2<usize>>, Vec<V2<usize>>) {
        let mut road_positions = vec![];
        let mut river_positions = vec![];
        let mut suppressed_river_positions = vec![];
        for x in from.x..to.x {
            for y in from.y..to.y {
                let position = v2(x, y);
                if let Some(cell) = world.get_cell(&position) {
                    let road = cell.road;
                    let river = cell.river;
                    if road.here() {
                        road_positions.push(position);
                    }
                    if river.here() {
                        if road.here() {
                            // We need these for drawing edges, but not nodes
                            suppressed_river_positions.push(position);
                        } else {
                            river_positions.push(position);
                        }
                    }
                }
            }
        }
        (road_positions, river_positions, suppressed_river_positions)
    }

    fn draw_slab_rivers_roads(&mut self, world: &World, slab: &Slab) -> Vec<Command> {
        let river_color = &Color::new(0.0, 0.0, 1.0, 1.0);
        let road_color = &Color::new(0.5, 0.5, 0.5, 1.0);
        let from = &slab.from;
        let to = &slab.to();
        let (road_positions, river_positions, suppressed_river_positions) =
            self.get_road_river_positions(world, from, to);
        let river_edges = river_positions
            .iter()
            .chain(suppressed_river_positions.iter())
            .flat_map(|position| {
                world
                    .get_cell(position)
                    .unwrap()
                    .river
                    .get_edges_from(position)
            })
            .collect();
        let road_edges = road_positions
            .iter()
            .flat_map(|position| {
                world
                    .get_cell(position)
                    .unwrap()
                    .road
                    .get_edges_from(position)
            })
            .collect();
        let mut out = vec![];
        out.append(&mut draw_edges(
            format!("{:?}-river-edges", slab.from),
            world,
            &river_edges,
            &river_color,
            world.sea_level(),
        ));
        out.append(&mut draw_edges(
            format!("{:?}-road-edges", slab.from),
            world,
            &road_edges,
            &road_color,
            world.sea_level(),
        ));
        out.append(&mut draw_nodes(
            format!("{:?}-river-positions", slab.from),
            world,
            &river_positions,
            &river_color,
            world.sea_level(),
        ));
        out.append(&mut draw_nodes(
            format!("{:?}-road-positions", slab.from),
            world,
            &road_positions,
            &road_color,
            world.sea_level(),
        ));
        out
    }

    fn draw_slabs(&mut self, world: &World, slabs: HashSet<Slab>) -> Vec<Command> {
        let mut out = vec![];
        for slab in slabs {
            out.append(&mut self.draw_slab(world, &slab));
        }
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
        out.append(&mut self.draw_terrain());
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

pub struct DefaultColoring {
    coloring: ShadedTileTerrainColoring,
}

impl DefaultColoring {
    pub fn new(
        world: &World,
        beach_level: f32,
        snow_temperature: f32,
        cliff_gradient: f32,
        light_direction: V3<f32>,
    ) -> DefaultColoring {
        DefaultColoring {
            coloring: ShadedTileTerrainColoring::new(
                Self::get_colors(world, beach_level, snow_temperature, cliff_gradient),
                Self::sea_color(),
                world.sea_level(),
                light_direction,
            ),
        }
    }

    fn sea_color() -> Color {
        Color::new(0.0, 0.0, 1.0, 1.0)
    }

    fn snow_color() -> Color {
        Color::new(1.0, 1.0, 1.0, 1.0)
    }

    fn cliff_color() -> Color {
        Color::new(0.5, 0.4, 0.3, 1.0)
    }

    fn beach_color() -> Color {
        Color::new(1.0, 1.0, 0.0, 1.0)
    }

    fn grass_color() -> Color {
        Color::new(0.0, 0.75, 0.0, 1.0)
    }

    fn get_colors(
        world: &World,
        beach_level: f32,
        snow_temperature: f32,
        cliff_gradient: f32,
    ) -> M<Color> {
        let width = world.width();
        let height = world.height();
        M::from_fn(width - 1, height - 1, |x, y| {
            Self::get_color(
                world,
                &v2(x, y),
                beach_level,
                snow_temperature,
                cliff_gradient,
            )
        })
    }

    fn get_color(
        world: &World,
        position: &V2<usize>,
        beach_level: f32,
        snow_temperature: f32,
        cliff_gradient: f32,
    ) -> Color {
        let max_gradient = world.get_max_abs_rise(&position);
        let min_elevation = world.get_lowest_corner(&position);
        if world
            .get_cell(position)
            .map(|cell| cell.climate.temperature <= snow_temperature)
            .unwrap_or(false)
        {
            Self::snow_color()
        } else if max_gradient > cliff_gradient {
            Self::cliff_color()
        } else if min_elevation < beach_level {
            Self::beach_color()
        } else {
            Self::grass_color()
        }
    }
}

impl TerrainColoring<WorldCell> for DefaultColoring {
    fn color(
        &self,
        world: &Grid<WorldCell>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        self.coloring.color(world, tile, triangle)
    }
}
