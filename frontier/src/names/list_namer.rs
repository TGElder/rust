use super::namer::Namer;
use commons::rand::prelude::*;
use std::fs::File;
use std::io;
use std::io::prelude::*;

pub struct ListNamer {
    names: Vec<String>,
    rng: SmallRng,
}

impl ListNamer {
    pub fn from_file(file: &str) -> ListNamer {
        ListNamer {
            names: names_from_file(file)
                .unwrap_or_else(|err| panic!("Could not read names from {:?}: {:?}", file, err)),
            rng: SeedableRng::from_entropy(),
        }
    }
}

fn names_from_file(file: &str) -> io::Result<Vec<String>> {
    let mut file = File::open(file)?;
    let mut contents = String::new();

    file.read_to_string(&mut contents)?;

    Ok(contents.split('\n').map(|line| line.to_string()).collect())
}

impl Namer for ListNamer {
    fn next_name(&mut self) -> String {
        self.names
            .choose(&mut self.rng)
            .expect("Cannot choose name from empty list")
            .clone()
    }
}
