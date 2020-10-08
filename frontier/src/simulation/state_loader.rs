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

    fn new_game(&mut self, game_state: &GameState) {
        let state = State {
            params: SimulationParams::default(),
            instructions: vec![],
            traffic: Vec2D::same_size_as(&game_state.world, HashSet::with_capacity(0)),
            edge_traffic: hashmap! {},
            route_to_ports: hashmap! {},
            build_queue: BuildQueue::default(),
            paused: false,
        };
        self.sim_tx.update(move |sim| sim.set_state(state));
    }

    fn load(&mut self, path: String) {
        self.sim_tx.update(move |sim| sim.load(&path));
    }

    fn init(&mut self) {
        self.sim_tx
            .update(move |sim| sim.start_processing_instructions());
    }
}

impl GameEventConsumer for SimulationStateLoader {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::NewGame => self.new_game(game_state),
            GameEvent::Load(path) => self.load(path.clone()),
            GameEvent::Init => self.init(),
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

    use crate::resource::Resource;
    use crate::route::RouteKey;
    use crate::world::World;
    use commons::edge::Edge;
    use commons::{v2, M};
    use std::collections::HashSet;
    use std::fs::remove_file;
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

    #[async_trait]
    impl Processor for StateRetriever {
        async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
            state.instructions.push(instruction.clone());
            self.tx.send(state).unwrap();
            State::default()
        }
    }

    #[test]
    fn new_game_event_should_set_sim_state() {
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
        let handle = thread::spawn(move || block_on(sim.run()));
        consumer.consume_game_event(&game_state, &GameEvent::NewGame);
        consumer.consume_game_event(&game_state, &GameEvent::Init);
        let state = state_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("State not retrieved after 10 seconds"));
        block_on(sim_tx.update(|sim| sim.shutdown()));
        handle.join().unwrap();

        // Then
        assert_eq!(
            state,
            State {
                params: SimulationParams::default(),
                instructions: vec![Instruction::Step],
                traffic: Vec2D::new(3, 7, HashSet::new()),
                edge_traffic: hashmap! {},
                route_to_ports: hashmap! {},
                build_queue: BuildQueue::default(),
                paused: false,
            }
        )
    }

    #[test]
    fn load_event_should_restore_sim_state() {
        // Given
        let file_name = "test_save.state_loader";

        let mut sim_1 = Simulation::new(vec![]);
        let route_key = RouteKey {
            settlement: v2(1, 2),
            resource: Resource::Crabs,
            destination: v2(3, 4),
        };
        let mut build_queue = BuildQueue::default();
        build_queue.insert(BuildInstruction {
            when: 808,
            what: Build::Road(Edge::new(v2(1, 2), v2(1, 3))),
        });
        let state = State {
            params: SimulationParams {
                traffic_to_population: 0.123,
                nation_flip_traffic_pc: 0.456,
                initial_town_population: 0.234,
                town_removal_population: 0.789,
            },
            instructions: vec![
                Instruction::GetTerritory(v2(1, 1)),
                Instruction::GetTerritory(v2(2, 2)),
                Instruction::GetTerritory(v2(3, 3)),
            ],
            traffic: Vec2D::new(3, 5, [route_key].iter().cloned().collect()),
            edge_traffic: hashmap! { Edge::new(v2(1, 2), v2(1, 3)) => hashset!{route_key} },
            route_to_ports: hashmap! { route_key => hashset!{ v2(1, 2), v2(3, 4) } },
            build_queue,
            paused: false,
        };
        sim_1.set_state(state.clone());
        sim_1.save(file_name);

        let (state_tx, state_rx) = channel();
        let retriever = StateRetriever::new(state_tx);
        let mut sim_2 = Simulation::new(vec![Box::new(retriever)]);
        let mut consumer = SimulationStateLoader::new(&sim_2.tx());

        // When
        let sim_tx = sim_2.tx().clone();
        let handle = thread::spawn(move || block_on(sim_2.run()));
        let game_state = GameState::default();
        consumer.consume_game_event(&game_state, &GameEvent::Load(file_name.to_string()));
        consumer.consume_game_event(&game_state, &GameEvent::Init);
        let retrieved = state_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("State not retrieved after 10 seconds"));
        block_on(sim_tx.update(|sim| sim.shutdown()));
        handle.join().unwrap();

        // Then
        assert_eq!(retrieved, state);

        // Finally
        remove_file(format!("{}.sim", file_name)).unwrap();
    }
}
