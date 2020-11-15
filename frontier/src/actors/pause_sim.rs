use crate::simulation::Simulation;
use commons::async_channel::{Receiver, RecvError};
use commons::fn_sender::FnSender;
use commons::futures::future::FutureExt;
use commons::log::debug;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

const NAME: &str = "pause_sim";

pub struct PauseSim {
    engine_rx: Receiver<Arc<Event>>,
    sim_tx: FnSender<Simulation>,
    binding: Button,
    run: bool,
}

impl PauseSim {
    pub fn new(engine_rx: Receiver<Arc<Event>>, sim_tx: &FnSender<Simulation>) -> PauseSim {
        PauseSim {
            engine_rx,
            sim_tx: sim_tx.clone_with_name(NAME),
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
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers: ModifiersState { alt: false, .. },
                ..
            } if *button == self.binding => self.pause().await,
            Event::Shutdown => self.shutdown().await,
            _ => (),
        }
    }

    async fn shutdown(&mut self) {
        self.run = false;
    }

    async fn pause(&mut self) {
        debug!("Pausing/resuming simulation");
        self.sim_tx
            .send(move |sim| sim.toggle_paused_persistent())
            .await;
        debug!("Paused/resumed simulation");
    }
}
