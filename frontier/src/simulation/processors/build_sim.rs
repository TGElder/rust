use super::*;

use crate::traits::Micros;

pub struct BuildSim<T> {
    tx: T,
    builders: Vec<Box<dyn Builder + Send>>,
}

#[async_trait]
impl<T> Processor for BuildSim<T>
where
    T: Micros + Send + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Build => (),
            _ => return state,
        };
        let micros = self.tx.micros().await;
        self.build_all(state.build_queue.take_instructions_before(micros))
            .await;
        state
    }
}

impl<T> BuildSim<T>
where
    T: Micros,
{
    pub fn new(tx: T, builders: Vec<Box<dyn Builder + Send>>) -> BuildSim<T> {
        BuildSim { tx, builders }
    }

    async fn build_all(&mut self, mut instructions: Vec<BuildInstruction>) {
        instructions.sort_by_key(|instruction| instruction.when);
        for BuildInstruction { what, .. } in instructions {
            self.build(what).await;
        }
    }

    async fn build(&mut self, build: Build) {
        for builder in self.builders.iter_mut() {
            if builder.can_build(&build) {
                builder.build(build).await;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::edge::Edge;
    use commons::v2;
    use futures::executor::block_on;
    use std::sync::{Arc, Mutex};
    struct BuildRetriever {
        builds: Arc<Mutex<Vec<Build>>>,
    }

    impl BuildRetriever {
        fn new() -> BuildRetriever {
            BuildRetriever {
                builds: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    #[async_trait]
    impl Builder for BuildRetriever {
        fn can_build(&self, _: &Build) -> bool {
            true
        }

        async fn build(&mut self, build: Build) {
            self.builds.lock().unwrap().push(build);
        }
    }

    #[async_trait]
    impl Micros for u128 {
        async fn micros(&self) -> u128 {
            *self
        }
    }

    #[test]
    fn should_hand_build_to_builder_if_when_elapsed() {
        // Given
        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut processor = BuildSim::new(1000, vec![Box::new(retriever)]);
        let mut state = State::default();
        state.build_queue.insert(BuildInstruction {
            what: Build::Road(Edge::new(v2(1, 2), v2(1, 3))),
            when: 100,
        });

        // When
        let state = block_on(processor.process(state, &Instruction::Build));

        // Then
        assert_eq!(
            *builds.lock().unwrap(),
            vec![Build::Road(Edge::new(v2(1, 2), v2(1, 3)))]
        );
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_hand_build_to_builder_if_when_not_elapsed() {
        // Given
        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut processor = BuildSim::new(1000, vec![Box::new(retriever)]);
        let instruction_1 = BuildInstruction {
            what: Build::Road(Edge::new(v2(1, 2), v2(1, 3))),
            when: 100,
        };
        let instruction_2 = BuildInstruction {
            what: Build::Road(Edge::new(v2(3, 4), v2(3, 5))),
            when: 2000,
        };
        let mut state = State::default();
        state.build_queue.insert(instruction_1);
        state.build_queue.insert(instruction_2.clone());

        // When
        let state = block_on(processor.process(state, &Instruction::Build));

        // Then
        assert_eq!(
            *builds.lock().unwrap(),
            vec![Build::Road(Edge::new(v2(1, 2), v2(1, 3)))]
        );
        let mut expected = BuildQueue::default();
        expected.insert(instruction_2);
        assert_eq!(state.build_queue, expected);
    }

    #[test]
    fn should_order_builds_by_when() {
        // Given
        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut processor = BuildSim::new(1000, vec![Box::new(retriever)]);
        let mut state = State::default();
        state.build_queue.insert(BuildInstruction {
            what: Build::Road(Edge::new(v2(1, 2), v2(1, 3))),
            when: 200,
        });
        state.build_queue.insert(BuildInstruction {
            what: Build::Road(Edge::new(v2(3, 4), v2(3, 5))),
            when: 100,
        });

        // When
        block_on(processor.process(state, &Instruction::Build));

        // Then
        assert_eq!(
            *builds.lock().unwrap(),
            vec![
                Build::Road(Edge::new(v2(3, 4), v2(3, 5))),
                Build::Road(Edge::new(v2(1, 2), v2(1, 3)))
            ]
        );
    }
}
