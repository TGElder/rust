use commons::rand::prelude::SmallRng;
use commons::rand::SeedableRng;

use crate::traits::has::HasParameters;
use crate::traits::WithWorld;
use crate::world_gen::generate_world;

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
        let mut rng: SmallRng = SeedableRng::seed_from_u64(params.seed);
        let mut generated_world = generate_world(params.power, &mut rng, &params.world_gen);
        if params.reveal_all {
            generated_world.reveal_all();
        }
        self.cx
            .mut_world(move |world| *world = generated_world)
            .await;
    }
}
