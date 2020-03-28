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
use std::sync::atomic::{AtomicBool, Ordering};
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

const HANDLE: &str = "simulation_runner";
const STEP_CHECK_DELAY: Duration = Duration::from_millis(100);
const LOAD_ORDERING: Ordering = Ordering::Relaxed;
const STORE_ORDERING: Ordering = Ordering::Relaxed;

pub trait Step {
    fn step(&mut self, year: u128);
}

struct Simulation {
    run: Arc<AtomicBool>,
    step: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
    steps: Vec<Box<dyn Step + Send>>,
    year: u128,
}

impl Simulation {
    fn run(&mut self) {
        while self.run.load(LOAD_ORDERING) {
            self.step();
        }
        self.done.store(true, STORE_ORDERING);
    }

    fn step(&mut self) {
        if !self.step.load(LOAD_ORDERING) {
            thread::sleep(STEP_CHECK_DELAY);
            return;
        }
        println!("Sim year {}", self.year);
        for step in &mut self.steps {
            step.step(self.year);
        }
        self.year += 1;
    }
}

pub struct SimulationRunner {
    run: Arc<AtomicBool>,
    step: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
    binding: Button,
}

impl SimulationRunner {
    pub fn new(start_year: u128, steps: Vec<Box<dyn Step + Send>>) -> SimulationRunner {
        let run = Arc::new(AtomicBool::new(true));
        let step = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));

        let mut simulation = Simulation {
            run: run.clone(),
            step: step.clone(),
            done: done.clone(),
            steps,
            year: start_year,
        };

        thread::spawn(move || simulation.run());

        SimulationRunner {
            run,
            step,
            done,
            binding: Button::Key(VirtualKeyCode::Y),
        }
    }
}

impl GameEventConsumer for SimulationRunner {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
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
                self.step
                    .store(!self.step.load(LOAD_ORDERING), STORE_ORDERING);
            }
        }
        CaptureEvent::No
    }

    fn shutdown(&mut self) {
        self.run.store(false, STORE_ORDERING);
    }

    fn is_shutdown(&self) -> bool {
        self.done.load(LOAD_ORDERING)
    }
}
