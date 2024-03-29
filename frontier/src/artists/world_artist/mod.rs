mod slab;

use commons::grid::Grid;
pub use slab::Slab;

use super::crop_artist::*;
use super::vegetation_artist::*;
use super::*;
use commons::*;
use isometric::drawing::*;
use isometric::*;

pub struct WorldColoring<'a> {
    pub terrain: Box<dyn TerrainColoring<WorldCell> + 'a>,
    pub crops: Box<dyn TerrainColoring<WorldCell> + 'a>,
}

#[derive(Clone)]
pub struct WorldArtistParameters {
    pub road_color: Color,
    pub river_color: Color,
    pub waterfall_color: Color,
    pub slab_size: usize,
    pub waterfall_gradient: f32,
}

impl Default for WorldArtistParameters {
    fn default() -> WorldArtistParameters {
        WorldArtistParameters {
            road_color: Color::new(0.6, 0.4, 0.0, 1.0),
            river_color: Color::new(0.0, 0.0, 1.0, 1.0),
            waterfall_color: Color::new(0.0, 0.75, 1.0, 1.0),
            slab_size: 64,
            waterfall_gradient: 0.1,
        }
    }
}

#[derive(Clone)]
pub struct WorldArtist {
    width: usize,
    height: usize,
    drawing: TerrainDrawing,
    vegetation_artist: VegetationArtist,
    crop_artist: CropArtist,
    params: WorldArtistParameters,
}

struct RoadRiverPositionsResult {
    road_positions: Vec<V2<usize>>,
    river_positions: Vec<V2<usize>>,
    suppressed_river_positions: Vec<V2<usize>>,
}

impl WorldArtist {
    pub fn new(width: usize, height: usize, params: WorldArtistParameters) -> WorldArtist {
        WorldArtist {
            width,
            height,
            drawing: TerrainDrawing::new("terrain".to_string(), width, height, params.slab_size),
            vegetation_artist: VegetationArtist::new(),
            crop_artist: CropArtist::new(),
            params,
        }
    }

    pub fn params(&self) -> &WorldArtistParameters {
        &self.params
    }

    pub fn init(&self) -> Vec<Command> {
        self.drawing.init()
    }

    pub fn draw_slab(&self, world: &World, coloring: &WorldColoring, slab: &Slab) -> Vec<Command> {
        let mut out = self.draw_slab_tiles(world, coloring, slab);
        out.append(&mut self.draw_slab_rivers_roads(world, slab));

        let from = slab.from;
        let to = slab.to();
        let to = v2(to.x.min(self.width - 1), to.y.min(self.height - 1));
        out.append(&mut self.draw_slab_crops(world, coloring, &from, &to));
        out.append(&mut self.draw_slab_vegetation(world, &from, &to));

        out
    }

    pub fn get_all_slabs(&self) -> Vec<Slab> {
        Slab::inside(&self.width, &self.height, &self.params.slab_size).collect()
    }

    fn draw_slab_tiles(
        &self,
        world: &World,
        coloring: &WorldColoring,
        slab: &Slab,
    ) -> Vec<Command> {
        let to = slab.to();
        let to = v2(to.x.min(self.width - 1), to.y.min(self.height - 1));
        self.drawing.update(
            world,
            world.sea_level(),
            coloring.terrain.as_ref(),
            slab.from,
            to,
        )
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
                    let platform = cell.platform;
                    let river = cell.river;
                    if road.here() || platform.here() {
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

    fn draw_slab_rivers_roads(&self, world: &World, slab: &Slab) -> Vec<Command> {
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
                    .get_rise(edge.from(), edge.to())
                    .map(|rise| rise.abs() <= self.params.waterfall_gradient)
                    .unwrap_or(true)
                    || (world.is_sea(edge.from()) && world.is_sea(edge.to()))
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

    fn draw_slab_crops(
        &self,
        world: &World,
        coloring: &WorldColoring,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Vec<Command> {
        self.crop_artist
            .draw(world, coloring.crops.as_ref(), from, to)
    }

    fn draw_slab_vegetation(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Vec<Command> {
        self.vegetation_artist.draw(world, from, to)
    }
}
