use crate::territory::Territory;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub struct TerritoryActor {
    state: Territory,
}

impl TerritoryActor {
    pub fn new(width: usize, height: usize) -> TerritoryActor {
        TerritoryActor {
            state: Territory::new(width, height),
        }
    }

    pub fn state(&mut self) -> &mut Territory {
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
        format!("{}.territory", path)
    }
}
