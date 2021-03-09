use commons::grid::Grid;
use commons::{v2, M};

use crate::services::Elevation;
use crate::traits::WithVisibility;
use crate::traits::WithWorld;
use crate::world::World;

pub struct SetupVisibility<T>
where
    T: WithVisibility + WithWorld,
{
    cx: T,
}

impl<T> SetupVisibility<T>
where
    T: WithVisibility + WithWorld,
{
    pub fn new(cx: T) -> SetupVisibility<T> {
        SetupVisibility { cx }
    }

    pub async fn init(&mut self) {
        let elevations = self.cx.with_world(|world| get_elevations(world)).await;
        self.cx
            .mut_visibility(|visibility| visibility.set_elevations(elevations))
            .await;
    }
}

fn get_elevations(world: &World) -> M<Elevation> {
    let sea_level = world.sea_level();
    M::from_fn(world.width(), world.height(), |x, y| Elevation {
        elevation: world.get_cell_unsafe(&v2(x, y)).elevation.max(sea_level),
    })
}
