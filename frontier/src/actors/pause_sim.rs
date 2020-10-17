use crate::simulation::Simulation;
use commons::async_channel::{Receiver, RecvError};
use commons::futures::future::FutureExt;
use commons::log::debug;
use commons::update::UpdateSender;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

const HANDLE: &str = "pause_sim";

pub struct PauseSim {
    engine_rx: Receiver<Arc<Event>>,
    sim_tx: UpdateSender<Simulation>,
    binding: Button,
    run: bool,
}

impl PauseSim {
    pub fn new(engine_rx: Receiver<Arc<Event>>, sim_tx: &UpdateSender<Simulation>) -> PauseSim {
        PauseSim {
            engine_rx,
            sim_tx: sim_tx.clone_with_handle(HANDLE),
            binding: Button::Key(VirtualKeyCode::Space),
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
            Event::Shutdown => self.shutdown().await,
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers: ModifiersState { alt: false, .. },
                ..
            } if *button == self.binding => self.pause().await,
            _ => (),
        }
    }

    async fn shutdown(&mut self) {
        self.run = false;
    }

    async fn pause(&mut self) {
        debug!("Pausing simulation");
        self.sim_tx
            .update(move |sim| sim.toggle_paused_persistent())
            .await;
        debug!("Paused simulation");
    }
}
