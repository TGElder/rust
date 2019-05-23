use commons::{M, V3};
use isometric::coords::WorldCoord;
use isometric::drawing::draw_house;
use isometric::Color;
use isometric::Command;

pub struct HouseBuilder {
    houses: M<bool>,
    light_direction: V3<f32>,
    color: Color,
}

impl HouseBuilder {
    pub fn new(width: usize, height: usize, light_direction: V3<f32>) -> HouseBuilder {
        HouseBuilder {
            houses: M::from_element(width, height, false),
            light_direction,
            color: Color::new(1.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn build_house(&mut self, world_coord: WorldCoord) -> Vec<Command> {
        let index = (world_coord.x as usize, world_coord.y as usize);
        self.houses[index] = !self.houses[index];
        let name = format!("house-{:?}", index);
        if self.houses[index] {
            draw_house(
                name,
                world_coord,
                0.25,
                0.5,
                0.5,
                self.color,
                self.light_direction,
            )
        } else {
            vec![Command::Erase(name)]
        }
    }
}
