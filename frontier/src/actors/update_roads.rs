use crate::game::Game;
use crate::road_builder::RoadBuilderResult;
use commons::fn_sender::{self, FnMessage, FnMessageExt, FnReceiver, FnSender, fn_channel};
use commons::futures::future::FutureExt;
use commons::{
    async_channel::unbounded,
    async_channel::{Receiver, RecvError, Sender},
};
use isometric::Event;
use std::sync::Arc;

use crate::actors::{Redraw, RedrawType, Visibility};

const HANDLE: &str = "update_roads";

pub struct UpdateRoads {
    tx: FnSender<UpdateRoads>,
    rx: FnReceiver<UpdateRoads>,
    subscribers: Vec<Sender<Arc<RoadBuilderResult>>>,
    engine_rx: Receiver<Arc<Event>>,
    game_tx: FnSender<Game>,
    redraw_tx: Sender<Redraw>,
    visibility_tx: FnSender<Visibility>,
    run: bool,
}

impl UpdateRoads {
    pub fn new(engine_rx: Receiver<Arc<Event>>, game_tx: &FnSender<Game>, redraw_tx: &Sender<Redraw>, visibility_tx: &FnSender<Visibility>) -> UpdateRoads {
        let (tx, rx) = fn_channel();
        UpdateRoads {
            tx,
            rx,
            subscribers: vec![],
            engine_rx,
            game_tx: game_tx.clone_with_name(HANDLE),
            redraw_tx: redraw_tx.clone(),
            visibility_tx: visibility_tx.clone_with_name(HANDLE),
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                message = self.rx.recv().fuse() => self.handle_message(message).await,
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event)
            }
        }
    }

    fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        if let Event::Shutdown = *event.unwrap() {
            self.shutdown();
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }

    async fn handle_message(&mut self, message: Result<FnMessage<UpdateRoads>, RecvError>) {
        if let Ok(mut message) = message {
            message.apply(self);
        }
    }

    fn update_roads(&mut self, result: RoadBuilderResult) {
        let result = Arc::new(result);
        let micros = self.update_world_get_micros(result.clone()).await;
        self.redraw(&result, micros).await;
        self.notify(&result).await;
    }

    async fn update_world_get_micros(&mut self, result: Arc<RoadBuilderResult>) -> u128 {
        self.game_tx
            .send(move |game| {result.update_roads(&mut game.mut_state().world); game.game_state().game_micros})
            .await
    }

    async fn redraw(&mut self, result: &Arc<RoadBuilderResult>, micros: u128) {
        for position in result.path() {
            self.redraw_tx.send(Redraw {
                redraw_type: RedrawType::Tile(*position),
                when: micros,
            }).await;
        }
    }

    async fn visit(&mut self, result: &Arc<RoadBuilderResult>) {
        let visited = result.path().iter().cloned().collect();
        self.visibility_tx.send(|visibility| visibility.check_visibility_and_reveal(visited));
    }

    async fn notify(&mut self, result: &Arc<RoadBuilderResult>) {
        for subscriber in self.subscribers.iter_mut() {
            subscriber.send(result.clone()).await.unwrap();
        }
    }
}
