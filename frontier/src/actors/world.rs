use commons::M;

use crate::traits::SendParameters;
use crate::world::World;
use crate::world_gen::{generate_world, rng};
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub struct WorldActor<T> {
    tx: T,
    state: World,
}

impl<T> WorldActor<T>
where
    T: SendParameters,
{
    pub fn new(tx: T) -> WorldActor<T> {
        WorldActor {
            tx,
            state: World::new(M::zeros(1, 1), 0.0),
        }
    }

    pub async fn new_game(&mut self) {
        self.state = self
            .tx
            .send_parameters(|params| {
                let mut rng = rng(params.seed);
                let mut world = generate_world(params.power, &mut rng, &params.world_gen);
                if params.reveal_all {
                    world.reveal_all();
                }
                world
            })
            .await;
    }

    pub fn state(&mut self) -> &mut World {
        &mut self.state
    }

    pub fn save(&self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
    }

    fn get_path(path: &str) -> String {
        format!("{}.world", path)
    }
}
