use std::collections::HashSet;

use commons::grid::Grid;
use commons::{v2, M, V2};
use isometric::cell_traits::WithElevation;

use crate::visibility_computer::VisibilityComputer;
use crate::world::World;

pub struct VisibilityService {
    computer: VisibilityComputer,
    elevations: Option<M<Elevation>>,
}

impl VisibilityService {
    pub fn new() -> VisibilityService {
        VisibilityService {
            computer: VisibilityComputer::default(),
            elevations: None,
        }
    }

    pub fn init(&mut self, world: &World) {
        self.elevations = Some(get_elevations(world));
    }

    pub fn get_visible_from(&self, position: V2<usize>) -> HashSet<V2<usize>> {
        self.computer
            .get_visible_from(self.elevations.as_ref().unwrap(), position)
    }
}

fn get_elevations(world: &World) -> M<Elevation> {
    let sea_level = world.sea_level();
    M::from_fn(world.width(), world.height(), |x, y| Elevation {
        elevation: world.get_cell_unsafe(&v2(x, y)).elevation.max(sea_level),
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Elevation {
    elevation: f32,
}

impl WithElevation for Elevation {
    fn elevation(&self) -> f32 {
        self.elevation
    }
}
