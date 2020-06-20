use super::*;

use crate::game::*;
use commons::update::*;
use isometric::Event;
use std::sync::Arc;

const HANDLE: &str = "simulation_state_loader";

pub struct SimulationStateLoader {
    sim_tx: UpdateSender<Simulation>,
}

impl SimulationStateLoader {
    pub fn new(sim_tx: &UpdateSender<Simulation>) -> SimulationStateLoader {
        SimulationStateLoader {
            sim_tx: sim_tx.clone_with_handle(HANDLE),
        }
    }

    fn load(&mut self, path: String) {
        self.sim_tx.update(move |sim| sim.load(&path));
    }
}

impl GameEventConsumer for SimulationStateLoader {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Load(path) = event {
            self.load(path.clone());
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
