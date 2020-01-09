use super::*;

use chrono::Duration as ChronoDuration;
use isometric::{Button, ElementState, VirtualKeyCode};

pub struct TimeLogger {
    binding: Button,
}

impl TimeLogger {
    pub fn new() -> TimeLogger {
        TimeLogger {
            binding: Button::Key(VirtualKeyCode::T),
        }
    }

    fn log_time(&self, game_state: &GameState) {
        println!("{:?}", Self::date_time_string(game_state));
    }

    fn date_time_string(game_state: &GameState) -> String {
        (game_state.params.history_start_date
            + ChronoDuration::microseconds(game_state.game_micros.try_into().unwrap()))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
    }
}

impl GameEventConsumer for TimeLogger {
    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            ..
        } = *event
        {
            if button == &self.binding {
                self.log_time(game_state);
            }
        }
        CaptureEvent::No
    }
}
