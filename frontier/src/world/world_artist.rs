use super::farm_artist::*;
use super::vegetation_artist::*;
use super::*;
use commons::*;
use isometric::drawing::*;
use isometric::*;
use std::collections::HashSet;

pub trait WorldColoring {
    fn terrain(&self) -> &dyn TerrainColoring<WorldCell>;
    fn farms(&self) -> &dyn TerrainColoring<WorldCell>;
}

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

pub struct WorldArtistParameters {
    pub road_color: Color,
    pub river_color: Color,
    pub waterfall_color: Color,
    pub slab_size: usize,
    pub vegetation_exageration: f32,
    pub waterfall_gradient: f32,
}

pub struct WorldArtist {
    width: usize,
    height: usize,
    drawing: TerrainDrawing,
    vegetation_artist: VegetationArtist,
    farm_artist: FarmArtist,
    params: WorldArtistParameters,
}

struct RoadRiverPositionsResult {
    road_positions: Vec<V2<usize>>,
    river_positions: Vec<V2<usize>>,
    suppressed_river_positions: Vec<V2<usize>>,
}

impl WorldArtist {
    pub fn new(world: &World, params: WorldArtistParameters) -> WorldArtist {
        let width = world.width();
        let height = world.height();
        WorldArtist {
            width,
            height,
            drawing: TerrainDrawing::new("terrain".to_string(), width, height, params.slab_size),
            vegetation_artist: VegetationArtist::new(params.vegetation_exageration),
            farm_artist: FarmArtist::new(),
            params,
        }
    }

    pub fn draw_terrain(&self) -> Vec<Command> {
        self.drawing.init()
    }

    fn draw_slab(
        &mut self,
        world: &World,
        coloring: &dyn WorldColoring,
        slab: &Slab,
    ) -> Vec<Command> {
        let mut out = self.draw_slab_tiles(world, coloring, slab);
        out.append(&mut self.draw_slab_rivers_roads(world, slab));
        out.append(&mut self.draw_slab_farms(world, coloring, slab));
        out.append(&mut self.draw_slab_vegetation(world, slab));
        out
    }

    fn draw_slab_tiles(
        &mut self,
        world: &World,
        coloring: &dyn WorldColoring,
        slab: &Slab,
    ) -> Vec<Command> {
        let to = slab.to();
        let to = v2(to.x.min(self.width - 1), to.y.min(self.height - 1));
        self.drawing
            .update(world, world.sea_level(), coloring.terrain(), slab.from, to)
    }

    fn get_road_river_positions(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> RoadRiverPositionsResult {
        let mut result = RoadRiverPositionsResult {
            road_positions: vec![],
            river_positions: vec![],
            suppressed_river_positions: vec![],
        };
        for x in from.x..to.x {
            for y in from.y..to.y {
                let position = v2(x, y);
                if let Some(cell) = world.get_cell(&position) {
                    let road = cell.road;
                    let river = cell.river;
                    if road.here() {
                        result.road_positions.push(position);
                    }
                    if river.here() {
                        if road.here() {
                            // We need these for drawing edges, but not nodes
                            result.suppressed_river_positions.push(position);
                        } else {
                            result.river_positions.push(position);
                        }
                    }
                }
            }
        }
        result
    }

    fn draw_slab_rivers_roads(&mut self, world: &World, slab: &Slab) -> Vec<Command> {
        let from = &slab.from;
        let to = &slab.to();
        let result = self.get_road_river_positions(world, from, to);
        let (river_edges, waterfall_edges): (Vec<Edge>, Vec<Edge>) = result
            .river_positions
            .iter()
            .chain(result.suppressed_river_positions.iter())
            .flat_map(|position| {
                world
                    .get_cell(position)
                    .unwrap()
                    .river
                    .get_edges_from(position)
            })
            .partition(|edge| {
                world
                    .get_rise(&edge.from(), &edge.to())
                    .map(|rise| rise.abs() <= self.params.waterfall_gradient)
                    .unwrap_or(true)
            });
        let road_edges: Vec<Edge> = result
            .road_positions
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
            &self.params.river_color,
            world.sea_level(),
        ));
        out.append(&mut draw_edges(
            format!("{:?}-waterfall-edges", slab.from),
            world,
            &waterfall_edges,
            &self.params.waterfall_color,
            world.sea_level(),
        ));
        out.append(&mut draw_edges(
            format!("{:?}-road-edges", slab.from),
            world,
            &road_edges,
            &self.params.road_color,
            world.sea_level(),
        ));
        out.append(&mut draw_nodes(
            format!("{:?}-river-positions", slab.from),
            world,
            &result.river_positions,
            &self.params.river_color,
            world.sea_level(),
        ));
        out.append(&mut draw_nodes(
            format!("{:?}-road-positions", slab.from),
            world,
            &result.road_positions,
            &self.params.road_color,
            world.sea_level(),
        ));
        out
    }

    fn draw_slab_farms(
        &mut self,
        world: &World,
        coloring: &dyn WorldColoring,
        slab: &Slab,
    ) -> Vec<Command> {
        let to = slab.to();
        let to = v2(to.x.min(self.width - 1), to.y.min(self.height - 1));
        self.farm_artist
            .draw(world, coloring.farms(), &slab.from, &to)
    }

    fn draw_slab_vegetation(&mut self, world: &World, slab: &Slab) -> Vec<Command> {
        let to = slab.to();
        let to = v2(to.x.min(self.width - 1), to.y.min(self.height - 1));
        self.vegetation_artist.draw(world, &slab.from, &to)
    }

    fn draw_slabs(
        &mut self,
        world: &World,
        coloring: &dyn WorldColoring,
        slabs: HashSet<Slab>,
    ) -> Vec<Command> {
        let mut out = vec![];
        for slab in slabs {
            out.append(&mut self.draw_slab(world, coloring, &slab));
        }
        out
    }

    fn get_affected_slabs(&self, world: &World, positions: &[V2<usize>]) -> HashSet<Slab> {
        positions
            .iter()
            .flat_map(|position| world.expand_position(&position))
            .map(|position| Slab::new(position, self.params.slab_size))
            .collect()
    }

    pub fn draw_affected(
        &mut self,
        world: &World,
        coloring: &dyn WorldColoring,
        positions: &[V2<usize>],
    ) -> Vec<Command> {
        let affected = self.get_affected_slabs(&world, positions);
        self.draw_slabs(world, coloring, affected)
    }

    fn get_all_slabs(&self) -> HashSet<Slab> {
        let mut out = HashSet::new();
        let slab_size = self.params.slab_size;
        for x in 0..self.width / slab_size {
            for y in 0..self.height / slab_size {
                let from = v2(x * slab_size, y * slab_size);
                out.insert(Slab::new(from, slab_size));
            }
        }
        out
    }

    pub fn draw_all(&mut self, world: &World, coloring: &dyn WorldColoring) -> Vec<Command> {
        self.draw_slabs(world, coloring, self.get_all_slabs())
    }

    pub fn init(&mut self, world: &World, coloring: &dyn WorldColoring) -> Vec<Command> {
        let mut out = vec![];
        out.append(&mut self.draw_terrain());
        out.append(&mut self.draw_all(world, coloring));
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
