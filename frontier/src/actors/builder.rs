use std::time::Duration;

use commons::async_std::task::sleep;
use commons::async_trait::async_trait;
use commons::process::Step;

use crate::build::{Build, BuildInstruction, Builder};
use crate::traits::{Micros, TakeBuildInstructionsBefore};

pub struct BuilderActor<T> {
    cx: T,
    builders: Vec<Box<dyn Builder + Send>>,
    build_interval: Duration,
}

impl<T> BuilderActor<T>
where
    T: Micros + TakeBuildInstructionsBefore,
{
    pub fn new(cx: T, builders: Vec<Box<dyn Builder + Send>>) -> BuilderActor<T> {
        BuilderActor {
            cx,
            builders,
            build_interval: Duration::from_millis(100),
        }
    }

    async fn build_all(&mut self, mut instructions: Vec<BuildInstruction>) {
        instructions.sort_by_key(|instruction| instruction.when);
        let mut build: Vec<Build> = instructions.into_iter().map(|BuildInstruction { what, .. }| what).collect();
        for builder in self.builders.iter_mut() {
            let (can_build, cannot_build): (Vec<Build>, Vec<Build>) = build.into_iter().partition(|what| builder.can_build(what));
            if !can_build.is_empty() {
                builder.build(can_build).await;
            }
            build = cannot_build;
        }
    }
}

#[async_trait]
impl<T> Step for BuilderActor<T>
where
    T: Micros + TakeBuildInstructionsBefore + Send + Sync,
{
    async fn step(&mut self) {
        let micros = self.cx.micros().await;
        self.build_all(self.cx.take_build_instructions_before(&micros).await)
            .await;
        sleep(self.build_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::edge::Edge;
    use commons::v2;
    use futures::executor::block_on;
    use std::sync::{Arc, Mutex};

    struct Cx {
        build_instructions: Vec<BuildInstruction>,
        micros: u128,
    }

    #[async_trait]
    impl Micros for Cx {
        async fn micros(&self) -> u128 {
            self.micros
        }
    }

    #[async_trait]
    impl TakeBuildInstructionsBefore for Cx {
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

        async fn build(&mut self, mut build: Vec<Build>) {
            self.builds.lock().unwrap().append(&mut build);
        }
    }

    #[test]
    fn should_pass_build_instructions_to_builders_ordered_by_when() {
        // Given
        let cx = Cx {
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

        let mut builder = BuilderActor::new(cx, vec![Box::new(retriever)]);

        // When
        block_on(builder.step());

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
