use std::time::Duration;

use commons::async_std::task::sleep;
use commons::async_trait::async_trait;
use commons::process::Step;

use crate::simulation::{Build, BuildInstruction, Builder};
use crate::traits::{Micros, TakeBuildInstructionsBefore};

pub struct BuildActor<T> {
    tx: T,
    builders: Vec<Box<dyn Builder + Send>>,
    build_interval: Duration,
}

impl<T> BuildActor<T>
where
    T: Micros + TakeBuildInstructionsBefore,
{
    pub fn new(tx: T, builders: Vec<Box<dyn Builder + Send>>) -> BuildActor<T> {
        BuildActor {
            tx,
            builders,
            build_interval: Duration::from_millis(100),
        }
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

#[async_trait]
impl<T> Step for BuildActor<T>
where
    T: Micros + TakeBuildInstructionsBefore + Send + Sync,
{
    async fn step(&mut self) {
        let micros = self.tx.micros().await;
        self.build_all(self.tx.take_build_instructions_before(&micros).await)
            .await;
        sleep(self.build_interval).await;
    }
}
