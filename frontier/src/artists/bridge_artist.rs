use commons::edge::Edge;
use commons::grid::Grid;
use commons::{v3, V3};
use isometric::drawing::draw_rectangle;
use isometric::{Color, Command};

use crate::bridge::Bridge;
use crate::world::World;

pub struct BridgeArtist {
    parameters: BridgeArtistParameters,
}

pub struct BridgeArtistParameters {
    pub color: Color,
    pub offset: f32,
}

impl BridgeArtist {
    pub fn new(parameters: BridgeArtistParameters) -> BridgeArtist {
        BridgeArtist { parameters }
    }

    pub fn draw_bridge(&self, world: &World, bridge: &Bridge) -> Vec<Command> {
        let coordinates = if bridge.edge.horizontal() {
            self.coordinates_horizontal(world, bridge)
        } else {
            self.coordinates_vertical(world, bridge)
        };
        let name = name(&bridge.edge);
        draw_rectangle(name, &coordinates, &self.parameters.color)
    }

    pub fn erase_bridge(&self, edge: &Edge) -> Command {
        Command::Erase(name(edge))
    }

    fn coordinates_horizontal(&self, world: &World, bridge: &Bridge) -> [V3<f32>; 4] {
        let from_z = world.get_cell_unsafe(bridge.edge.from()).elevation;
        let to_z = world.get_cell_unsafe(bridge.edge.to()).elevation;
        let offset = self.parameters.offset;
        [
            v3(
                bridge.edge.from().x as f32 + offset,
                bridge.edge.from().y as f32 - offset,
                from_z,
            ),
            v3(
                bridge.edge.from().x as f32 + offset,
                bridge.edge.from().y as f32 + offset,
                from_z,
            ),
            v3(
                bridge.edge.to().x as f32 - offset,
                bridge.edge.to().y as f32 + offset,
                to_z,
            ),
            v3(
                bridge.edge.to().x as f32 - offset,
                bridge.edge.to().y as f32 - offset,
                to_z,
            ),
        ]
    }

    fn coordinates_vertical(&self, world: &World, bridge: &Bridge) -> [V3<f32>; 4] {
        let from_z = world.get_cell_unsafe(bridge.edge.from()).elevation;
        let to_z = world.get_cell_unsafe(bridge.edge.to()).elevation;
        let offset = self.parameters.offset;
        [
            v3(
                bridge.edge.from().x as f32 + offset,
                bridge.edge.from().y as f32 + offset,
                from_z,
            ),
            v3(
                bridge.edge.from().x as f32 - offset,
                bridge.edge.from().y as f32 + offset,
                from_z,
            ),
            v3(
                bridge.edge.to().x as f32 - offset,
                bridge.edge.to().y as f32 - offset,
                to_z,
            ),
            v3(
                bridge.edge.to().x as f32 + offset,
                bridge.edge.to().y as f32 - offset,
                to_z,
            ),
        ]
    }
}

fn name(edge: &Edge) -> String {
    format!(
        "bridge-{},{}-{},{}",
        edge.from().x,
        edge.from().y,
        edge.to().x,
        edge.to().y
    )
}