use commons::async_trait::async_trait;

use crate::traits::SendWorld;

#[async_trait]
pub trait VisibleLandPositions {
    async fn visible_land_positions(&self) -> usize;
}

#[async_trait]
impl<T> VisibleLandPositions for T
where
    T: SendWorld + Send + Sync,
{
    async fn visible_land_positions(&self) -> usize {
        self.send_world(|world| {
            world
                .cells()
                .filter(|cell| cell.visible)
                .filter(|cell| !world.is_sea(&cell.position))
                .count()
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use commons::grid::Grid;
    use commons::{v2, M};
    use futures::executor::block_on;

    use crate::world::World;

    use super::*;

    #[async_trait]
    impl SendWorld for Mutex<World> {
        async fn send_world<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut crate::world::World) -> O + Send + 'static,
        {
            function(&mut self.lock().unwrap())
        }

        fn send_world_background<F, O>(&self, function: F)
        where
            O: Send + 'static,
            F: FnOnce(&mut crate::world::World) -> O + Send + 'static,
        {
            function(&mut self.lock().unwrap());
        }
    }

    #[test]
    fn test_visible_land_positions() {
        let mut world = World::new(M::from_fn(3, 3, |x, _| if x == 1 { 1.0 } else { 0.0 }), 0.5);
        world.mut_cell_unsafe(&v2(0, 0)).visible = true;
        world.mut_cell_unsafe(&v2(0, 1)).visible = true;
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;

        assert_eq!(block_on(Mutex::new(world).visible_land_positions()), 2);
    }
}
