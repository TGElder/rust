use crate::game::Game;
use crate::road_builder::RoadBuilderResult;
use commons::fn_sender::FnSender;
use commons::futures::future::FutureExt;
use commons::{
    async_channel::unbounded,
    async_channel::{Receiver, RecvError, Sender},
};
use isometric::Event;
use std::sync::Arc;

const HANDLE: &str = "update_roads";

pub struct UpdateRoads {
    tx: Sender<RoadBuilderResult>,
    rx: Receiver<RoadBuilderResult>,
    subscribers: Vec<Sender<Arc<RoadBuilderResult>>>,
    engine_rx: Receiver<Arc<Event>>,
    game_tx: FnSender<Game>,
    run: bool,
}

impl UpdateRoads {
    pub fn new(engine_rx: Receiver<Arc<Event>>, game_tx: &FnSender<Game>) -> UpdateRoads {
        let (tx, rx) = unbounded();
        UpdateRoads {
            tx,
            rx,
            subscribers: vec![],
            engine_rx,
            game_tx: game_tx.clone_with_name(HANDLE),
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                message = self.rx.recv().fuse() => self.handle_message(message).await,
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        if let Event::Shutdown = *event.unwrap() {
            self.shutdown();
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }

    async fn handle_message(&mut self, message: Result<RoadBuilderResult, RecvError>) {
        if let Ok(result) = message {
            let result = Arc::new(result);
            self.update_world(result.clone()).await;
            self.notify(result).await;
        }
    }

    async fn update_world(&mut self, result: Arc<RoadBuilderResult>) {
        self.game_tx
            .send(move |game| result.update_roads(&mut game.mut_state().world))
            .await;
    }

    async fn notify(&mut self, result: Arc<RoadBuilderResult>) {
        for subscriber in self.subscribers.iter_mut() {
            subscriber.send(result.clone()).await.unwrap();
        }
    }
}
