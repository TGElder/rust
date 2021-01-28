use crate::route::Routes;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub struct RoutesActor {
    state: Routes,
}

impl RoutesActor {
    pub fn new() -> RoutesActor {
        RoutesActor {
            state: Routes::default(),
        }
    }

    pub fn state(&mut self) -> &mut Routes {
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
        format!("{}.routes", path)
    }
}
