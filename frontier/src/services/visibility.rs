use std::collections::HashSet;

use commons::{M, V2};
use isometric::cell_traits::WithElevation;

use crate::visibility_computer::VisibilityComputer;

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

    pub fn set_elevations(&mut self, elevations: M<Elevation>) {
        self.elevations = Some(elevations);
    }

    pub fn get_visible_from(&self, position: V2<usize>) -> HashSet<V2<usize>> {
        self.computer
            .get_visible_from(self.elevations.as_ref().unwrap(), position)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Elevation {
    pub elevation: f32,
}

impl WithElevation for Elevation {
    fn elevation(&self) -> f32 {
        self.elevation
    }
}
