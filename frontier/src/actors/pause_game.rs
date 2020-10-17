use crate::game::Game;
use commons::async_channel::{Receiver, RecvError};
use commons::futures::future::FutureExt;
use commons::log::debug;
use commons::update::UpdateSender;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

const HANDLE: &str = "pause_game";

pub struct PauseGame {
    engine_rx: Receiver<Arc<Event>>,
    game_tx: UpdateSender<Game>,
    binding: Button,
    run: bool,
}

impl PauseGame {
    pub fn new(engine_rx: Receiver<Arc<Event>>, game_tx: &UpdateSender<Game>) -> PauseGame {
        PauseGame {
            engine_rx,
            game_tx: game_tx.clone_with_handle(HANDLE),
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
        debug!("Pausing game");
        self.game_tx.update(move |sim| toggle_pause(sim)).await;
        debug!("Paused game");
    }
}

fn toggle_pause(game: &mut Game) {
    let game_state = game.mut_state();
    if game_state.speed == 0.0 {
        game_state.speed = game_state.params.default_speed;
    } else {
        game_state.speed = 0.0;
    }
}
