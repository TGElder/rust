use crate::process::{Persistable, Step};
use crate::traits::SendWorld;

use super::*;

use commons::index2d::Vec2D;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub struct Simulation<X> {
    x: X,
    processors: Vec<Box<dyn Processor + Send>>,
    state: Option<State>,
}

impl<X> Simulation<X>
where
    X: SendWorld + Send,
{
    pub fn new(x: X, processors: Vec<Box<dyn Processor + Send>>) -> Simulation<X> {
        Simulation {
            x,
            processors,
            state: None,
        }
    }

    pub async fn new_game(&mut self) {
        self.state = Some(State {
            params: SimulationParams::default(),
            instructions: vec![],
            traffic: self
                .x
                .send_world(|world| Vec2D::same_size_as(world, HashSet::with_capacity(0)))
                .await,
            edge_traffic: hashmap! {},
            route_to_ports: hashmap! {},
            build_queue: BuildQueue::default(),
        });
    }

    pub fn refresh_positions(&mut self, positions: HashSet<V2<usize>>) {
        if let Some(state) = &mut self.state {
            state
                .instructions
                .push(Instruction::RefreshPositions(positions));
        }
    }

    pub fn update_homeland_population(&mut self) {
        if let Some(state) = &mut self.state {
            state
                .instructions
                .push(Instruction::UpdateHomelandPopulation);
        }
    }

    async fn process_instruction(&mut self, mut state: State) -> State {
        if let Some(instruction) = state.instructions.pop() {
            for processor in self.processors.iter_mut() {
                state = processor.process(state, &instruction).await;
            }
        }
        state
    }

    fn try_step(&mut self, state: &mut State) {
        if state.instructions.is_empty() {
            state.instructions.push(Instruction::Step);
        }
    }
}

#[async_trait]
impl<X> Step for Simulation<X>
where
    X: SendWorld + Send,
{
    async fn step(&mut self) {
        let state = unwrap_or!(self.state.take(), return);
        let mut state = self.process_instruction(state).await;
        self.try_step(&mut state);
        self.state = Some(state);
    }
}

impl<X> Persistable for Simulation<X> {
    fn save(&self, path: &str) {
        let path = get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    fn load(&mut self, path: &str) {
        let path = get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
    }
}

fn get_path(path: &str) -> String {
    format!("{}.sim", path)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::route::RouteKey;
    use crate::world::World;
    use commons::edge::Edge;
    use commons::index2d::Vec2D;
    use commons::{v2, Arm, M};
    use futures::executor::block_on;
    use std::fs::remove_file;
    use std::sync::{Arc, Mutex};

    fn world() -> Arm<World> {
        Arc::new(Mutex::new(World::new(M::zeros(3, 3), 0.0)))
    }

    #[async_trait]
    impl SendWorld for Arm<World> {
        async fn send_world<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut World) -> O + Send + 'static,
        {
            function(&mut self.lock().unwrap())
        }

        fn send_world_background<F, O>(&self, function: F)
        where
            O: Send + 'static,
            F: FnOnce(&mut World) -> O + Send + 'static,
        {
            function(&mut self.lock().unwrap());
        }
    }

    #[test]
    fn should_hand_instructions_to_all_processors() {
        // Given
        struct InstructionRetriever {
            instructions: Arm<Vec<Instruction>>,
        }

        #[async_trait]
        impl Processor for InstructionRetriever {
            async fn process(&mut self, state: State, instruction: &Instruction) -> State {
                self.instructions.lock().unwrap().push(instruction.clone());
                state
            }
        }

        let instructions_1 = Arm::default();
        let instructions_2 = Arm::default();
        let retriever_1 = InstructionRetriever {
            instructions: instructions_1.clone(),
        };
        let retriever_2 = InstructionRetriever {
            instructions: instructions_2.clone(),
        };
        let mut sim = Simulation::new(world(), vec![Box::new(retriever_1), Box::new(retriever_2)]);
        sim.state = Some(State {
            instructions: vec![Instruction::Step],
            ..State::default()
        });

        // When
        block_on(sim.step());

        // Then
        assert_eq!(*instructions_1.lock().unwrap(), vec![Instruction::Step]);
        assert_eq!(*instructions_2.lock().unwrap(), vec![Instruction::Step]);
    }

    #[test]
    fn should_add_step_instruction_if_instructions_are_empty() {
        // Given
        let mut sim = Simulation::new(world(), vec![]);
        sim.state = Some(State::default());

        // When
        block_on(sim.step());

        // Then
        assert_eq!(sim.state.unwrap().instructions, vec![Instruction::Step]);
    }

    #[test]
    fn processors_should_be_able_to_update_state() {
        // Given
        struct InstructionIntroducer {}

        #[async_trait]
        impl Processor for InstructionIntroducer {
            async fn process(&mut self, mut state: State, _: &Instruction) -> State {
                state.instructions.push(Instruction::Build);
                state
            }
        }

        let mut sim = Simulation::new(world(), vec![Box::new(InstructionIntroducer {})]);
        sim.state = Some(State {
            instructions: vec![Instruction::Step],
            ..State::default()
        });

        // When
        block_on(sim.step());

        // Then
        assert_eq!(sim.state.unwrap().instructions, vec![Instruction::Build]);
    }

    #[test]
    fn traffic_should_be_initialised_same_size_as_world_with_empty_maps() {
        let mut sim = Simulation::new(world(), vec![]);
        block_on(sim.new_game());
        assert_eq!(
            sim.state.unwrap().traffic,
            Vec2D::new(3, 3, HashSet::with_capacity(0))
        );
    }

    #[test]
    fn save_load_round_trip() {
        // Given
        let file_name = "test_save.simulation.round_trip";

        let mut sim_1 = Simulation::new(world(), vec![]);
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
        sim_1.state = Some(State {
            params: SimulationParams {
                road_build_threshold: 8,
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
        });
        sim_1.save(file_name);

        // When
        let mut sim_2 = Simulation::new(world(), vec![]);
        sim_2.load(file_name);

        // Then
        assert_eq!(sim_1.state, sim_2.state);

        // Finally
        remove_file(format!("{}.sim", file_name)).unwrap();
    }
}
