use crate::avatar::*;
use crate::game::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use crate::territory::*;
use crate::world::*;
use commons::edge::*;
use commons::futures::executor::block_on;
use commons::update::*;
use commons::V2;
use isometric::Event;
use isometric::{Button, ElementState, VirtualKeyCode};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

mod children;
mod commuter;
mod farm_assigner;
mod natural_road;
mod natural_town;
mod params;
mod territory;

pub use children::*;
pub use commuter::*;
pub use farm_assigner::*;
pub use natural_road::*;
pub use natural_town::*;
pub use params::*;
pub use territory::*;

const STEP_CHECK_DELAY: Duration = Duration::from_millis(100);
const UPDATE_CHANNEL_BOUND: usize = 100;

pub trait Step {
    fn step(&mut self, year: u128);
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct SimulationState {
    year: u128,
}

pub struct Simulation {
    steps: Vec<Box<dyn Step + Send>>,
    update_tx: UpdateSender<Simulation>,
    update_rx: UpdateReceiver<Simulation>,
    state: SimulationState,
    run: bool,
    step: bool,
}

impl Simulation {
    pub fn new(start_year: u128, steps: Vec<Box<dyn Step + Send>>) -> Simulation {
        let (update_tx, update_rx) = update_channel(UPDATE_CHANNEL_BOUND);

        Simulation {
            steps,
            update_tx,
            update_rx,
            state: SimulationState { year: start_year },
            run: true,
            step: false,
        }
    }
    pub fn update_tx(&self) -> &UpdateSender<Simulation> {
        &self.update_tx
    }

    pub fn shutdown(&mut self) {
        self.run = false;
    }

    pub fn run(&mut self) {
        while self.run {
            self.step();
        }
    }

    fn step(&mut self) {
        if self.step {
            let year = &mut self.state.year;
            println!("Year {}", year);
            for step in &mut self.steps {
                step.step(*year);
            }
            *year += 1;
        } else {
            thread::sleep(STEP_CHECK_DELAY);
        }
        let updates = self.update_rx.get_updates();
        process_updates(updates, self);
    }

    fn get_path(path: &str) -> String {
        format!("{}.sim", path)
    }

    pub fn save(&mut self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
    }
}

const HANDLE: &str = "simulation_manager";

pub struct SimulationManager {
    sim_tx: UpdateSender<Simulation>,
    binding: Button,
}

impl SimulationManager {
    pub fn new(sim_tx: &UpdateSender<Simulation>) -> SimulationManager {
        SimulationManager {
            sim_tx: sim_tx.clone_with_handle(HANDLE),
            binding: Button::Key(VirtualKeyCode::Y),
        }
    }

    fn load(&mut self, path: String) {
        self.sim_tx.update(move |sim| sim.load(&path));
    }
}

impl GameEventConsumer for SimulationManager {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Load(path) = event {
            self.load(path.clone());
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            ..
        } = *event
        {
            if button == &self.binding {
                self.sim_tx.update(|sim| sim.step = !sim.step);
            }
        }
        CaptureEvent::No
    }
}
