mod game_params;
mod game_state;

use commons::log::warn;
pub use game_params::*;
pub use game_state::*;

use commons::fn_sender::*;
use futures::executor::block_on;
use std::time::{Duration, Instant};

pub struct Game {
    game_state: GameState,
    tx: FnSender<Game>,
    rx: FnReceiver<Game>,
    run: bool,
}

impl Game {
    pub fn new(game_state: GameState) -> Game {
        let (tx, rx) = fn_channel();

        Game {
            game_state,
            tx,
            rx,
            run: true,
        }
    }

    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    pub fn tx(&self) -> &FnSender<Game> {
        &self.tx
    }

    pub fn save(&mut self, path: String) {
        self.game_state.to_file(&path);
    }

    pub fn run(&mut self) {
        while self.run {
            for message in self.rx.get_messages() {
                self.handle_message(message);
            }
        }
    }

    fn handle_message(&mut self, mut message: FnMessage<Game>) {
        let start = Instant::now();
        let name = message.sender_name();
        block_on(message.apply(self));
        log_time(
            name.to_string(),
            start.elapsed(),
            &self.game_state.params.log_duration_threshold,
        );
    }

    pub fn shutdown(&mut self) {
        self.run = false;
    }
}

fn log_time(description: String, duration: Duration, threshold: &Option<Duration>) {
    if let Some(threshold) = threshold {
        if duration >= *threshold {
            warn!("{},{}ms", description, duration.as_millis());
        }
    }
}
