use crate::world::*;
use commons::*;
use isometric::coords::WorldCoord;
use isometric::drawing::{draw_house, DrawHouseParams};
use isometric::Color;
use isometric::Command;

pub struct HouseBuilder {
    houses: M<bool>,
    params: DrawHouseParams,
}

impl HouseBuilder {
    pub fn new(houses: M<bool>, light_direction: V3<f32>) -> HouseBuilder {
        HouseBuilder {
            houses,
            params: DrawHouseParams {
                width: 0.25,
                height: 0.5,
                roof_height: 0.5,
                basement_z: 0.0,
                base_color: Color::new(1.0, 0.0, 0.0, 1.0),
                light_direction,
            },
        }
    }

    pub fn houses(&self) -> &M<bool> {
        &self.houses
    }

    pub fn build_house(&mut self, position: &V2<usize>, world: &World) -> Vec<Command> {
        let index = (position.x, position.y);
        let world_coord = world.snap_to_middle(WorldCoord::new(
            position.x as f32,
            position.y as f32,
            0 as f32,
        ));
        if let Some(world_coord) = world_coord {
            let basement_z = world.get_lowest_corner(position);
            self.houses[index] = !self.houses[index];
            let name = format!("house-{:?}", index);
            if self.houses[index] {
                draw_house(
                    name,
                    world_coord,
                    &DrawHouseParams {
                        basement_z,
                        ..self.params
                    },
                )
            } else {
                vec![Command::Erase(name)]
            }
        } else {
            return vec![];
        }
    }

    pub fn rebuild_houses(&mut self, world: &World) -> Vec<Command> {
        self.house_list()
            .iter()
            .flat_map(|house| {
                *self.houses.mut_cell_unsafe(house) = false;
                self.build_house(house, world)
            })
            .collect()
    }

    fn house_list(&self) -> Vec<V2<usize>> {
        let mut out = vec![];
        let (width, height) = self.houses.shape();
        for x in 0..width {
            for y in 0..height {
                if self.houses[(x, y)] {
                    out.push(v2(x, y));
                }
            }
        }
        out
    }
}
