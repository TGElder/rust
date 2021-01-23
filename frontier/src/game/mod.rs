mod game_params;
mod game_state;
pub mod traits;

use commons::log::warn;
pub use game_params::*;
pub use game_state::*;

use crate::avatar::*;
use commons::fn_sender::*;
use commons::V2;
use commons::*;
use futures::executor::block_on;
use std::collections::hash_map::Entry;
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
    avatar_travel_duration: AvatarTravelDuration,
    run: bool,
}

impl Game {
    pub fn new(game_state: GameState) -> Game {
        let (tx, rx) = fn_channel();

        Game {
            previous_instant: Instant::now(),
            avatar_travel_duration: AvatarTravelDuration::with_planned_roads_ignored(
                &game_state.params.avatar_travel,
            ),
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

    fn on_tick(&mut self) {
        self.update_game_micros();
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

    pub fn walk_positions(&mut self, name: String, positions: Vec<V2<usize>>, start_at: u128) {
        let start_at = start_at.max(self.game_state.game_micros);
        if let Entry::Occupied(mut avatar) = self.game_state.avatars.all.entry(name) {
            let journey = avatar.get_mut().journey.take().unwrap();
            if let Some(new_journey) = journey.append(Journey::new(
                &self.game_state.world,
                positions,
                &self.avatar_travel_duration,
                self.avatar_travel_duration.travel_mode_fn(),
                start_at,
            )) {
                avatar.get_mut().journey = Some(new_journey);
            }
        }
    }

    pub fn save(&mut self, path: String) {
        self.game_state.to_file(&path);
    }

    pub fn run(&mut self) {
        while self.run {
            self.on_tick();
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
