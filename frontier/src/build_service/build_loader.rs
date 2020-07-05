use super::*;

use crate::game::traits::Micros;
use crate::game::*;
use isometric::Event;
use std::sync::Arc;

const HANDLE: &str = "build_queue_loader";

pub struct BuildQueueLoader<G>
where
    G: Micros,
{
    builder_tx: UpdateSender<BuildService<G>>,
}

impl<G> BuildQueueLoader<G>
where
    G: Micros,
{
    pub fn new(builder_tx: &UpdateSender<BuildService<G>>) -> BuildQueueLoader<G> {
        BuildQueueLoader {
            builder_tx: builder_tx.clone_with_handle(HANDLE),
        }
    }

    fn load(&mut self, path: String) {
        self.builder_tx.update(move |sim| sim.load(&path));
    }
}

impl<G> GameEventConsumer for BuildQueueLoader<G>
where
    G: Micros,
{
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Load(path) = event {
            self.load(path.clone())
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

    use commons::futures::executor::block_on;
    use commons::update::UpdateProcess;
    use commons::v2;
    use std::fs::remove_file;
    use std::sync::mpsc::{channel, Sender};
    use std::thread;
    use std::time::Duration;

    struct BuildRetriever {
        tx: Sender<Build>,
    }

    impl BuildRetriever {
        fn new(tx: Sender<Build>) -> BuildRetriever {
            BuildRetriever { tx }
        }
    }

    impl Builder for BuildRetriever {
        fn can_build(&self, _: &Build) -> bool {
            true
        }

        fn build(&mut self, build: Build) {
            self.tx.send(build).unwrap()
        }
    }

    #[test]
    fn load_event_should_restore_build_queue() {
        // Given
        let file_name = "test_save.build_loader";

        let game = UpdateProcess::new(1000);
        let mut build_service_1 = BuildService::new(&game.tx(), vec![]);
        build_service_1.queue(BuildInstruction {
            what: Build::Road(v2(1, 2)),
            when: 200,
        });
        build_service_1.save(file_name);

        let (build_tx, build_rx) = channel();
        let retriever = BuildRetriever::new(build_tx);
        let mut build_service_2 = BuildService::new(&game.tx(), vec![Box::new(retriever)]);
        let mut consumer = BuildQueueLoader::new(&build_service_2.tx());

        // When
        let builder_tx = build_service_2.tx().clone();
        let handle = thread::spawn(move || build_service_2.run());
        consumer.consume_game_event(
            &GameState::default(),
            &GameEvent::Load(file_name.to_string()),
        );
        let built = build_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("Build not retrieved after 10 seconds"));
        block_on(async { builder_tx.update(|builder| builder.shutdown()).await });
        handle.join().unwrap();

        // Then
        assert_eq!(built, Build::Road(v2(1, 2)));

        // Finally
        game.shutdown();
        remove_file(format!("{}.build_service", file_name)).unwrap();
    }
}
