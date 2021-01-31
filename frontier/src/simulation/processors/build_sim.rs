use super::*;

use crate::traits::{Micros, TakeBuildInstructionsBefore};

pub struct BuildSim<T> {
    tx: T,
    builders: Vec<Box<dyn Builder + Send>>,
}

#[async_trait]
impl<T> Processor for BuildSim<T>
where
    T: Micros + TakeBuildInstructionsBefore + Send + Sync,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Build => (),
            _ => return state,
        };
        let micros = self.tx.micros().await;
        self.build_all(self.tx.take_build_instructions_before(&micros).await)
            .await;
        state
    }
}

impl<T> BuildSim<T>
where
    T: Micros + TakeBuildInstructionsBefore,
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

    struct Tx {
        build_instructions: Vec<BuildInstruction>,
        micros: u128,
    }

    #[async_trait]
    impl Micros for Tx {
        async fn micros(&self) -> u128 {
            self.micros
        }
    }

    #[async_trait]
    impl TakeBuildInstructionsBefore for Tx {
        async fn take_build_instructions_before(&self, _: &u128) -> Vec<BuildInstruction> {
            self.build_instructions.clone()
        }
    }

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

    #[test]
    fn should_pass_build_instructions_to_builders_ordered_by_when() {
        // Given
        let tx = Tx {
            build_instructions: vec![
                BuildInstruction {
                    what: Build::Road(Edge::new(v2(1, 2), v2(1, 3))),
                    when: 200,
                },
                BuildInstruction {
                    what: Build::Road(Edge::new(v2(3, 4), v2(3, 5))),
                    when: 100,
                },
            ],
            micros: 1000,
        };
        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut processor = BuildSim::new(tx, vec![Box::new(retriever)]);

        // When
        block_on(processor.process(State::default(), &Instruction::Build));

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
