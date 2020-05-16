use super::equalize_ignoring_sea;
use crate::world::World;
use commons::grid::Grid;
use commons::{v2, M};

pub fn compute_groundwater(world: &World) -> M<f32> {
    equalize_ignoring_sea(
        M::from_fn(world.width(), world.height(), |x, y| {
            groundwater_at(world, x, y)
        }),
        world,
    )
}

fn groundwater_at(world: &World, x: usize, y: usize) -> f32 {
    let position = v2(x, y);
    let cell = world.get_cell_unsafe(&position);
    cell.climate.river_water + cell.climate.rainfall
}

pub fn load_groundwater(world: &mut World, groundwater: &M<f32>) {
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = v2(x, y);
            world.mut_cell_unsafe(&position).climate.groundwater =
                *groundwater.get_cell_unsafe(&position);
        }
    }
}
