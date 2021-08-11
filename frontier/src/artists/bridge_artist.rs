use commons::{v3, V3};
use isometric::drawing::draw_rectangle;
use isometric::{Color, Command};

use crate::bridges::{Bridge, Segment};

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

    pub fn draw_bridge(&self, bridge: &Bridge) -> Vec<Command> {
        bridge
            .segments()
            .flat_map(|segment| self.draw_segment(bridge, segment).into_iter())
            .collect()
    }

    pub fn draw_segment(&self, bridge: &Bridge, segment: Segment) -> Vec<Command> {
        if segment.from.position == segment.to.position {
            return vec![];
        }
        let coordinates = if segment.edge().horizontal() {
            self.coordinates_horizontal(&segment)
        } else {
            self.coordinates_vertical(&segment)
        };
        let name = name(&bridge, segment);
        draw_rectangle(name, &coordinates, &self.parameters.color)
    }

    fn coordinates_horizontal(&self, segment: &Segment) -> [V3<f32>; 4] {
        let from = &segment.from;
        let to = &segment.to;
        let offset = self.parameters.offset;
        [
            v3(
                from.position.x as f32 + if segment.from.platform { offset } else { 0.0 },
                from.position.y as f32 - offset,
                from.elevation,
            ),
            v3(
                from.position.x as f32 + if segment.from.platform { offset } else { 0.0 },
                from.position.y as f32 + offset,
                from.elevation,
            ),
            v3(
                to.position.x as f32 - if segment.to.platform { offset } else { 0.0 },
                to.position.y as f32 + offset,
                to.elevation,
            ),
            v3(
                to.position.x as f32 - if segment.to.platform { offset } else { 0.0 },
                to.position.y as f32 - offset,
                to.elevation,
            ),
        ]
    }

    fn coordinates_vertical(&self, segment: &Segment) -> [V3<f32>; 4] {
        let from = &segment.from;
        let to = &segment.to;
        let offset = self.parameters.offset;
        [
            v3(
                from.position.x as f32 + offset,
                from.position.y as f32 + if segment.from.platform { offset } else { 0.0 },
                from.elevation,
            ),
            v3(
                from.position.x as f32 - offset,
                from.position.y as f32 + if segment.from.platform { offset } else { 0.0 },
                from.elevation,
            ),
            v3(
                to.position.x as f32 - offset,
                to.position.y as f32 - if segment.to.platform { offset } else { 0.0 },
                to.elevation,
            ),
            v3(
                to.position.x as f32 + offset,
                to.position.y as f32 - if segment.to.platform { offset } else { 0.0 },
                to.elevation,
            ),
        ]
    }

    pub fn erase_bridge(&self, bridge: &Bridge) -> Vec<Command> {
        bridge
            .segments()
            .map(|segment| self.erase_segment(bridge, segment))
            .collect()
    }

    pub fn erase_segment(&self, bridge: &Bridge, segment: Segment) -> Command {
        Command::Erase(name(bridge, segment))
    }
}

fn name(bridge: &Bridge, segment: Segment) -> String {
    let bridge_edge = bridge.total_edge();
    let segment_edge = segment.edge();
    format!(
        "bridge-{},{}-{},{}/{}-{},{}-{}",
        bridge_edge.from().x,
        bridge_edge.from().y,
        bridge_edge.to().x,
        bridge_edge.to().y,
        segment_edge.from().x,
        segment_edge.from().y,
        segment_edge.to().x,
        segment_edge.to().y
    )
}
