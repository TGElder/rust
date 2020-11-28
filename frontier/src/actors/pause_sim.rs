use crate::traits::SendSim;
use commons::async_channel::{Receiver, RecvError};
use commons::futures::future::FutureExt;
use commons::log::debug;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

pub struct PauseSim<T>
where
    T: SendSim,
{
    x: T,
    engine_rx: Receiver<Arc<Event>>,
    binding: Button,
    run: bool,
}

impl<T> PauseSim<T>
where
    T: SendSim,
{
    pub fn new(x: T, engine_rx: Receiver<Arc<Event>>) -> PauseSim<T> {
        PauseSim {
            x,
            engine_rx,
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
        self.x
            .send_sim(move |sim| sim.toggle_paused_persistent())
            .await;
        debug!("Paused/resumed simulation");
    }
}
