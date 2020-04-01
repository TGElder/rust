use super::*;
use isometric::{Button, ElementState, VirtualKeyCode};

const HANDLE: &str = "speed_control";
const SECONDS_PER_HOUR: f32 = 3600.0;

pub struct SpeedControlBindings {
    slow_down: Button,
    speed_up: Button,
}

impl Default for SpeedControlBindings {
    fn default() -> SpeedControlBindings {
        SpeedControlBindings {
            slow_down: Button::Key(VirtualKeyCode::Comma),
            speed_up: Button::Key(VirtualKeyCode::Period),
        }
    }
}

pub struct SpeedControl {
    game_tx: UpdateSender<Game>,
    hours_per_second: [f32; 13],
    index: usize,
    bindings: SpeedControlBindings,
}

impl SpeedControl {
    pub fn new(game_tx: &UpdateSender<Game>) -> SpeedControl {
        SpeedControl {
            game_tx: game_tx.clone_with_handle(HANDLE),
            hours_per_second: [
                0.0,
                0.000_277_778, // Real time
                0.0625,
                0.125,
                0.25,
                0.5,
                1.0,
                2.0,
                4.0,
                8.0,
                16.0,
                32.0,
                64.0,
            ],
            index: 6,
            bindings: SpeedControlBindings::default(),
        }
    }

    fn slow_down(&mut self) {
        if self.index > 0 {
            self.index -= 1;
            self.update_speed();
        }
    }

    fn speed_up(&mut self) {
        if self.index < self.hours_per_second.len() - 1 {
            self.index += 1;
            self.update_speed();
        }
    }

    fn update_speed(&mut self) {
        let speed = self.hours_per_second[self.index] * SECONDS_PER_HOUR;
        self.game_tx.update(move |game: &mut Game| {
            game.mut_state().speed = speed;
            println!("speed = {}", game.game_state().speed);
        });
    }
}

impl GameEventConsumer for SpeedControl {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Init = event {
            self.update_speed();
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            ..
        } = *event
        {
            if button == &self.bindings.slow_down {
                self.slow_down();
            }
            if button == &self.bindings.speed_up {
                self.speed_up();
            }
        }
        CaptureEvent::No
    }
}
