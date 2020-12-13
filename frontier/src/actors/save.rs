use crate::system::Persistable;
use crate::traits::{SendGame, SendSim};
use commons::async_channel::{Receiver, RecvError};
use commons::futures::future::FutureExt;
use commons::log::debug;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

const PATH: &str = "save";

pub struct Save<T> {
    x: T,
    engine_rx: Receiver<Arc<Event>>,
    binding: Button,
    path: String,
    run: bool,
}

impl<T> Save<T>
where
    T: SendGame + SendSim,
{
    pub fn new(x: T, engine_rx: Receiver<Arc<Event>>) -> Save<T> {
        Save {
            x,
            engine_rx,
            binding: Button::Key(VirtualKeyCode::P),
            path: PATH.to_string(),
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        let event: Arc<Event> = event.unwrap();
        match *event {
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers:
                    ModifiersState {
                        alt: false,
                        ctrl: true,
                        ..
                    },
                ..
            } if *button == self.binding => self.save().await,
            Event::Shutdown => self.shutdown().await,
            _ => (),
        }
    }

    async fn shutdown(&mut self) {
        self.run = false;
    }

    async fn save(&mut self) {
        let path_for_sim = self.path.clone();
        let path_for_game = self.path.clone();
        self.x.send_sim(move |sim| sim.pause()).await;
        debug!("Paused simulation");
        self.x.send_sim(move |sim| sim.save(&path_for_sim)).await;
        debug!("Saved simulation state");
        self.x.send_game(|game| game.save(path_for_game)).await;
        debug!("Saved game state");
        self.x.send_sim(move |sim| sim.resume()).await;
        debug!("Resumed simulation");
    }
}
