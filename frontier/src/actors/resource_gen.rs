use commons::rand::prelude::SmallRng;
use commons::rand::SeedableRng;

use crate::resource_gen::ResourceGen;
use crate::traits::has::HasParameters;
use crate::traits::{WithResources, WithWorld};

pub struct ResourceGenActor<T> {
    cx: T,
}

impl<T> ResourceGenActor<T>
where
    T: HasParameters + WithResources + WithWorld,
{
    pub fn new(cx: T) -> ResourceGenActor<T> {
        ResourceGenActor { cx }
    }

    pub async fn new_game(&mut self) {
        let params = self.cx.parameters();
        let mut rng: SmallRng = SeedableRng::seed_from_u64(params.seed);

        let new_resources = self
            .cx
            .with_world(|world| {
                ResourceGen::new(params.power, world, params, &mut rng).compute_resources()
            })
            .await;

        self.cx
            .mut_resources(move |resources| *resources = new_resources)
            .await;
    }
}
