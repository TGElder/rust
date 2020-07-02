use super::*;

use crate::game::*;
use commons::index2d::Vec2D;
use commons::update::*;
use isometric::Event;
use std::collections::HashSet;
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

    fn init(&mut self, game_state: &GameState) {
        let state = State {
            instructions: vec![],
            traffic: Vec2D::same_size_as(&game_state.world, HashSet::with_capacity(0)),
        };
        self.sim_tx.update(move |sim| sim.set_state(state));
    }

    fn load(&mut self, path: String) {
        self.sim_tx.update(move |sim| sim.load(&path));
    }
}

impl GameEventConsumer for SimulationStateLoader {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::Load(path) => self.load(path.clone()),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::route::RouteKey;
    use crate::world::{Resource, World};
    use commons::{v2, M};
    use std::collections::HashSet;
    use std::sync::mpsc::{channel, Sender};
    use std::thread;
    use std::time::Duration;

    struct StateRetriever {
        tx: Sender<State>,
    }

    impl StateRetriever {
        fn new(tx: Sender<State>) -> StateRetriever {
            StateRetriever { tx }
        }
    }

    impl Processor for StateRetriever {
        fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
            state.instructions.push(instruction.clone());
            self.tx.send(state).unwrap();
            State::default()
        }
    }

    #[test]
    fn init_event_should_set_sim_state() {
        // Given
        let (state_tx, state_rx) = channel();
        let retriever = StateRetriever::new(state_tx);
        let mut sim = Simulation::new(vec![Box::new(retriever)]);
        let mut consumer = SimulationStateLoader::new(&sim.tx());
        let game_state = GameState {
            world: World::new(M::zeros(3, 7), 0.5),
            ..GameState::default()
        };

        // When
        let sim_tx = sim.tx().clone();
        let handle = thread::spawn(move || sim.run());
        consumer.consume_game_event(&game_state, &GameEvent::Init);
        let state = state_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("State not retrieved after 10 seconds"));
        block_on(async { sim_tx.update(|sim| sim.shutdown()).await });
        handle.join().unwrap();

        // Then
        assert_eq!(
            state,
            State {
                instructions: vec![Instruction::Step],
                traffic: Vec2D::new(3, 7, HashSet::new())
            }
        )
    }

    #[test]
    fn load_event_should_restore_sim_state() {
        // Given
        let mut sim_1 = Simulation::new(vec![]);
        let route_key = RouteKey {
            settlement: v2(1, 2),
            resource: Resource::Crabs,
            destination: v2(3, 4),
        };
        let state = State {
            instructions: vec![
                Instruction::SettlementRef(v2(1, 1)),
                Instruction::SettlementRef(v2(2, 2)),
                Instruction::SettlementRef(v2(3, 3)),
            ],
            traffic: Vec2D::new(3, 5, [route_key].iter().cloned().collect()),
        };
        sim_1.set_state(state.clone());
        sim_1.save("test_save");

        let (state_tx, state_rx) = channel();
        let retriever = StateRetriever::new(state_tx);
        let mut sim_2 = Simulation::new(vec![Box::new(retriever)]);
        let mut consumer = SimulationStateLoader::new(&sim_2.tx());

        // When
        let sim_tx = sim_2.tx().clone();
        let handle = thread::spawn(move || sim_2.run());
        consumer.consume_game_event(
            &GameState::default(),
            &GameEvent::Load("test_save".to_string()),
        );
        let retrieved = state_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("State not retrieved after 10 seconds"));
        block_on(async { sim_tx.update(|sim| sim.shutdown()).await });
        handle.join().unwrap();

        // Then
        assert_eq!(retrieved, state);
    }
}
