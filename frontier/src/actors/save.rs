use crate::game::Game;
use crate::simulation::Simulation;
use commons::async_channel::{Receiver, RecvError};
use commons::futures::future::FutureExt;
use commons::log::debug;
use commons::update::UpdateSender;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

const HANDLE: &str = "save";
const PATH: &str = "save";

pub struct Save {
    engine_rx: Receiver<Arc<Event>>,
    game_tx: UpdateSender<Game>,
    sim_tx: UpdateSender<Simulation>,
    binding: Button,
    path: String,
    run: bool,
}

impl Save {
    pub fn new(
        engine_rx: Receiver<Arc<Event>>,
        game_tx: &UpdateSender<Game>,
        sim_tx: &UpdateSender<Simulation>,
    ) -> Save {
        Save {
            engine_rx,
            game_tx: game_tx.clone_with_handle(HANDLE),
            sim_tx: sim_tx.clone_with_handle(HANDLE),
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
                modifiers: ModifiersState { alt: false, .. },
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
        self.sim_tx.update(move |sim| sim.pause()).await;
        debug!("Paused simulation");
        self.sim_tx.update(move |sim| sim.save(&path_for_sim)).await;
        debug!("Saved simulation state");
        self.game_tx.update(|game| game.save(path_for_game)).await;
        debug!("Saved game state");
        self.sim_tx.update(move |sim| sim.resume()).await;
        debug!("Resumed simulation");
    }
}
