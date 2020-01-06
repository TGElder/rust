use super::*;
use isometric::{Button, ElementState, VirtualKeyCode};

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
    command_tx: Sender<GameCommand>,
    bindings: SpeedControlBindings,
    hours_per_second: [f32; 11],
    index: usize,
}

impl SpeedControl {
    pub fn new(command_tx: Sender<GameCommand>) -> SpeedControl {
        SpeedControl {
            command_tx,
            bindings: SpeedControlBindings::default(),
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
            ],
            index: 6,
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
        let function: Box<dyn FnOnce(&mut GameState) -> Vec<GameCommand> + Send> =
            Box::new(move |game_state| {
                game_state.speed = speed;
                vec![]
            });
        self.command_tx.send(GameCommand::Update(function)).unwrap();
    }
}

impl GameEventConsumer for SpeedControl {
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
