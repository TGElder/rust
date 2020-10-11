use super::*;

use crate::game::traits::Micros;

const HANDLE: &str = "builder";

pub struct BuildSim<G>
where
    G: Micros,
{
    game: UpdateSender<G>,
    builders: Vec<Box<dyn Builder + Send>>,
}

#[async_trait]
impl<G> Processor for BuildSim<G>
where
    G: Micros,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Build => (),
            _ => return state,
        };
        let micros = self.micros().await;
        self.build_all(state.build_queue.take_instructions_before(micros))
            .await;
        state
    }
}

impl<G> BuildSim<G>
where
    G: Micros,
{
    pub fn new(game: &UpdateSender<G>, builders: Vec<Box<dyn Builder + Send>>) -> BuildSim<G> {
        BuildSim {
            game: game.clone_with_handle(HANDLE),
            builders,
        }
    }

    async fn micros(&mut self) -> u128 {
        self.game.update(|game| *game.micros()).await
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
    use commons::futures::executor::block_on;
    use commons::update::UpdateProcess;
    use commons::v2;
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

    #[test]
    fn should_hand_build_to_builder_if_when_elapsed() {
        // Given
        let game = UpdateProcess::new(1000);

        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut processor = BuildSim::new(&game.tx(), vec![Box::new(retriever)]);
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
        let game = UpdateProcess::new(1000);

        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut processor = BuildSim::new(&game.tx(), vec![Box::new(retriever)]);
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
        let game = UpdateProcess::new(1000);

        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut processor = BuildSim::new(&game.tx(), vec![Box::new(retriever)]);
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
