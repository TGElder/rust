use super::*;
use isometric::{Button, ElementState, VirtualKeyCode};

const HANDLE: &str = "speed_control";

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
    game_tx: FnSender<Game>,
    bindings: SpeedControlBindings,
}

impl SpeedControl {
    pub fn new(game_tx: &FnSender<Game>) -> SpeedControl {
        SpeedControl {
            game_tx: game_tx.clone_with_name(HANDLE),
            bindings: SpeedControlBindings::default(),
        }
    }

    fn slow_down(&mut self) {
        self.game_tx.send(move |game: &mut Game| {
            game.mut_state().speed /= 2.0;
            println!("speed = {}", game.game_state().speed);
        });
    }

    fn speed_up(&mut self) {
        self.game_tx.send(move |game: &mut Game| {
            game.mut_state().speed *= 2.0;
            println!("speed = {}", game.game_state().speed);
        });
    }
}

impl GameEventConsumer for SpeedControl {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
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
