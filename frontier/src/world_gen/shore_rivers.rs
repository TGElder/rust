use commons::{V2, v2};
use commons::grid::Grid;

use crate::world::World;

pub fn add_shore_rivers(world: &mut World, width: f32) {
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = v2(x, y);
            if is_shore(world, &position) {
                let river = &mut world.mut_cell_unsafe(&position).river;
                river.horizontal.width = width;
                river.vertical.width = width;
            }
        }
    }
}

fn is_shore(world: &World, position: &V2<usize>) -> bool {
    world.is_sea(position) && world.neighbours(position).iter().any(|neighbour| !world.is_sea(neighbour))
}