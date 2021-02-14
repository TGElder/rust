use crate::traits::has::HasParameters;
use crate::traits::WithWorld;
use crate::world_gen::{generate_world, rng};

pub struct WorldGen<T> {
    tx: T,
}

impl<T> WorldGen<T>
where
    T: HasParameters + WithWorld,
{
    pub fn new(tx: T) -> WorldGen<T> {
        WorldGen { tx }
    }

    pub async fn new_game(&mut self) {
        let params = self.tx.parameters();
        let mut rng = rng(params.seed);
        let mut new_world = generate_world(params.power, &mut rng, &params.world_gen);
        if params.reveal_all {
            new_world.reveal_all();
        }
        self.tx.mut_world(move |world| *world = new_world).await;
    }
}
