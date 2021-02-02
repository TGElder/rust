use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub trait Save {
    fn save(&self, path: &str);
}

impl<T> Save for T
where
    T: Serialize,
{
    fn save(&self, path: &str) {
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self).unwrap();
    }
}

pub trait Load {
    fn load(path: &str) -> Self;
}

impl<T> Load for T
where
    T: DeserializeOwned,
{
    fn load(path: &str) -> Self {
        let file = BufReader::new(File::open(path).unwrap());
        bincode::deserialize_from(file).unwrap()
    }
}
