use crate::traits::has::HasParameters;
use crate::traits::WithWorld;
use crate::world_gen::{generate_world, rng};

pub struct WorldGen<T> {
    cx: T,
}

impl<T> WorldGen<T>
where
    T: HasParameters + WithWorld,
{
    pub fn new(cx: T) -> WorldGen<T> {
        WorldGen { cx }
    }

    pub async fn new_game(&mut self) {
        let params = self.cx.parameters();
        let mut rng = rng(params.seed);
        let mut new_world = generate_world(params.power, &mut rng, &params.world_gen);
        if params.reveal_all {
            new_world.reveal_all();
        }
        self.cx.mut_world(move |world| *world = new_world).await;
    }
}
