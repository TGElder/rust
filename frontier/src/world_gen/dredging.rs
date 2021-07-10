use std::collections::HashMap;

use commons::grid::Grid;
use commons::log::debug;
use commons::{unsafe_ordering, v2, V2};

use crate::world::World;

pub fn dredge(world: &mut World) {
    while try_dredge(world) > 0 {}
}

fn try_dredge(world: &mut World) -> usize {
    let horizontal = [v2(-1, 0), v2(1, 0)];
    let vertical = [v2(0, -1), v2(0, 1)];
    let mut dredged = 0;
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = v2(x, y);
            if try_dredge_position(world, &position, &horizontal) {
                dredged += 1;
            }
            if try_dredge_position(world, &position, &vertical) {
                dredged += 1;
            }
        }
    }
    debug!("Dredged {} positions", dredged);
    dredged
}

fn try_dredge_position(world: &mut World, position: &V2<usize>, offsets: &[V2<i32>]) -> bool {
    let sea_level = world.sea_level();

    let elevation = world.get_cell_unsafe(position).elevation;

    if elevation > sea_level {
        return false;
    }

    let neighbours: HashMap<V2<usize>, f32> = offsets
        .iter()
        .flat_map(|offset| world.offset(position, *offset))
        .map(|neighbour| (neighbour, world.get_cell_unsafe(&neighbour).elevation))
        .collect();

    if neighbours.is_empty() {
        return false;
    }

    if neighbours
        .iter()
        .any(|(_, elevation)| *elevation <= sea_level)
    {
        return false;
    }

    let lowest_neighbour = neighbours
        .into_iter()
        .min_by(|a, b| unsafe_ordering(&a.1, &b.1))
        .map(|(a, _)| a)
        .unwrap();

    world.mut_cell_unsafe(&lowest_neighbour).elevation = elevation;

    true
}
