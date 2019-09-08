use super::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

pub struct Save {
    command_tx: Sender<GameCommand>,
    binding: Button,
    path: String,
}

impl Save {
    pub fn new(command_tx: Sender<GameCommand>) -> Save {
        Save {
            command_tx,
            binding: Button::Key(VirtualKeyCode::P),
            path: "save".to_string(),
        }
    }

    fn save(&self, game_state: &GameState) {
        game_state.to_file(&self.path);
        self.command_tx
            .send(GameCommand::Event(GameEvent::Save(self.path.clone())))
            .unwrap();
    }
}

impl GameEventConsumer for Save {
    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                self.save(game_state);
            }
        }
        CaptureEvent::No
    }
}
