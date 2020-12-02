use crate::game::Game;
use commons::async_channel::{Receiver, RecvError};
use commons::fn_sender::FnSender;
use commons::futures::future::FutureExt;
use commons::log::debug;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

const NAME: &str = "pause_game";

pub struct PauseGame {
    engine_rx: Receiver<Arc<Event>>,
    game_tx: FnSender<Game>,
    binding: Button,
    run: bool,
}

impl PauseGame {
    pub fn new(engine_rx: Receiver<Arc<Event>>, game_tx: &FnSender<Game>) -> PauseGame {
        PauseGame {
            engine_rx,
            game_tx: game_tx.clone_with_name(NAME),
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
            Event::Shutdown => self.shutdown(),
            _ => (),
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }

    async fn pause(&mut self) {
        debug!("Pausing/resuming game");
        self.game_tx.send(move |sim| toggle_pause(sim)).await;
        debug!("Paused/resumed game");
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
