use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::time::Instant;

pub struct Micros {
    baseline_instant: Instant,
    state: MicrosState,
}

#[derive(Serialize, Deserialize)]
struct MicrosState {
    baseline_micros: u128,
    speed: f32,
}

impl Micros {
    pub fn new(speed: f32) -> Micros {
        Micros {
            baseline_instant: Instant::now(),
            state: MicrosState {
                baseline_micros: 0,
                speed,
            },
        }
    }

    pub fn init(&mut self) {
        self.baseline_instant = Instant::now();
    }

    pub fn get_micros(&self) -> u128 {
        self.get_micros_at(&Instant::now())
    }

    fn get_micros_at(&self, instant: &Instant) -> u128 {
        self.state.baseline_micros
            + (instant.duration_since(self.baseline_instant).as_micros() as f32 * self.state.speed)
                .round() as u128
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.update_baseline();
        self.state.speed = speed;
    }

    fn update_baseline(&mut self) {
        let new_baseline_instant = Instant::now();
        self.state.baseline_micros = self.get_micros_at(&new_baseline_instant);
        self.baseline_instant = new_baseline_instant;
    }

    pub fn save(&mut self, path: &str) {
        self.update_baseline();
        let path = get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
    }
}

fn get_path(path: &str) -> String {
    format!("{}.micros", path)
}
