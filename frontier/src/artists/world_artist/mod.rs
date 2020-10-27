mod slab;

pub use slab::Slab;

use super::crop_artist::*;
use super::resource_artist::*;
use super::vegetation_artist::*;
use super::*;
use commons::*;
use isometric::drawing::*;
use isometric::*;
use std::collections::HashSet;

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
    pub vegetation: VegatationArtistParams,
    pub resource: ResourceArtistParameters,
}

impl Default for WorldArtistParameters {
    fn default() -> WorldArtistParameters {
        WorldArtistParameters {
            road_color: Color::new(0.6, 0.4, 0.0, 1.0),
            river_color: Color::new(0.0, 0.0, 1.0, 1.0),
            waterfall_color: Color::new(0.0, 0.75, 1.0, 1.0),
            slab_size: 64,
            waterfall_gradient: 0.1,
            vegetation: VegatationArtistParams::default(),
            resource: ResourceArtistParameters::default(),
        }
    }
}

#[derive(Clone)]
pub struct WorldArtist {
    width: usize,
    height: usize,
    drawing: TerrainDrawing,
    vegetation_artist: VegetationArtist,
    resource_artist: ResourceArtist,
    crop_artist: CropArtist,
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
            vegetation_artist: VegetationArtist::new(params.vegetation),
            resource_artist: ResourceArtist::new(params.resource),
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
        out.append(&mut self.draw_slab_resources(world, &from, &to));

        out
    }

    pub fn get_all_slabs(&self) -> HashSet<Slab> {
        let mut out = HashSet::new();
        let slab_size = self.params.slab_size;
        for x in 0..self.width / slab_size {
            for y in 0..self.height / slab_size {
                let from = v2(x * slab_size, y * slab_size);
                out.insert(Slab::at(from, slab_size));
            }
        }
        out
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

    fn draw_slab_resources(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Vec<Command> {
        self.resource_artist.draw(world, from, to)
    }
}
