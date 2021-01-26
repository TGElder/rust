mod game_params;
mod game_state;
pub mod traits;

use commons::log::warn;
pub use game_params::*;
pub use game_state::*;

use commons::fn_sender::*;
use commons::V2;
use commons::*;
use futures::executor::block_on;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub struct TerritoryState {
    pub controller: V2<usize>,
    pub durations: HashMap<V2<usize>, Duration>,
}

pub struct Game {
    game_state: GameState,
    previous_instant: Instant,
    tx: FnSender<Game>,
    rx: FnReceiver<Game>,
    run: bool,
}

impl Game {
    pub fn new(game_state: GameState) -> Game {
        let (tx, rx) = fn_channel();

        Game {
            previous_instant: Instant::now(),
            game_state,
            tx,
            rx,
            run: true,
        }
    }

    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    pub fn mut_state(&mut self) -> &mut GameState {
        &mut self.game_state
    }

    pub fn tx(&self) -> &FnSender<Game> {
        &self.tx
    }

    fn update_game_micros(&mut self) {
        let current_instant = Instant::now();
        let interval = current_instant
            .duration_since(self.previous_instant)
            .as_micros();
        let interval = (interval as f32 * self.game_state.speed).round();
        self.game_state.game_micros += interval as u128;
        self.previous_instant = current_instant;
    }

    pub fn save(&mut self, path: String) {
        self.game_state.to_file(&path);
    }

    pub fn run(&mut self) {
        while self.run {
            for message in self.rx.get_messages() {
                self.update_game_micros();
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
