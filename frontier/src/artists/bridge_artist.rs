use commons::edge::Edge;
use commons::grid::Grid;
use commons::{v3, V3};
use isometric::drawing::{create_plain, draw_rectangle};
use isometric::{Color, Command};

use crate::bridge::Bridge;
use crate::world::World;

pub struct BridgeArtist {
    color: Color,
    offset: f32,
}

impl BridgeArtist {
    pub fn new(color: Color, offset: f32) -> BridgeArtist {
        BridgeArtist { color, offset }
    }

    pub fn draw_bridge(&self, world: &World, bridge: &Bridge) -> Vec<Command> {
        let coordinates = if bridge.edge.horizontal() {
            self.coordinates_horizontal(world, bridge)
        } else {
            self.coordinates_vertical(world, bridge)
        };
        let name = name(&bridge.edge);
        vec![
            create_plain(name.clone(), 36), // TODO add create and draw function?
            draw_rectangle(name, &coordinates, &self.color),
        ]
    }

    pub fn erase_bridge(&self, edge: &Edge) -> Command {
        Command::Erase(name(edge))
    }

    fn coordinates_horizontal(&self, world: &World, bridge: &Bridge) -> [V3<f32>; 4] {
        let from_z = world.get_cell_unsafe(bridge.edge.from()).elevation;
        let to_z = world.get_cell_unsafe(bridge.edge.to()).elevation;
        [
            v3(
                bridge.edge.from().x as f32 + self.offset,
                bridge.edge.from().y as f32 - self.offset,
                from_z,
            ),
            v3(
                bridge.edge.from().x as f32 + self.offset,
                bridge.edge.from().y as f32 + self.offset,
                from_z,
            ),
            v3(
                bridge.edge.to().x as f32 - self.offset,
                bridge.edge.to().y as f32 + self.offset,
                to_z,
            ),
            v3(
                bridge.edge.to().x as f32 - self.offset,
                bridge.edge.to().y as f32 - self.offset,
                to_z,
            ),
        ]
    }

    fn coordinates_vertical(&self, world: &World, bridge: &Bridge) -> [V3<f32>; 4] {
        let from_z = world.get_cell_unsafe(bridge.edge.from()).elevation;
        let to_z = world.get_cell_unsafe(bridge.edge.to()).elevation;
        [
            v3(
                bridge.edge.from().x as f32 + self.offset,
                bridge.edge.from().y as f32 + self.offset,
                from_z,
            ),
            v3(
                bridge.edge.from().x as f32 - self.offset,
                bridge.edge.from().y as f32 + self.offset,
                from_z,
            ),
            v3(
                bridge.edge.to().x as f32 - self.offset,
                bridge.edge.to().y as f32 - self.offset,
                to_z,
            ),
            v3(
                bridge.edge.to().x as f32 + self.offset,
                bridge.edge.to().y as f32 - self.offset,
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
