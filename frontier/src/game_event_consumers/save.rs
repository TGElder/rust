use super::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "save";

pub struct Save {
    game_tx: UpdateSender<Game>,
    binding: Button,
    path: String,
}

impl Save {
    pub fn new(game_tx: &UpdateSender<Game>) -> Save {
        Save {
            game_tx: game_tx.clone_with_handle(HANDLE),
            binding: Button::Key(VirtualKeyCode::P),
            path: "save".to_string(),
        }
    }

    fn save(&mut self) {
        let path = self.path.clone();
        self.game_tx.update(|game| game.save(path));
    }
}

impl GameEventConsumer for Save {
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
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                self.save();
            }
        }
        CaptureEvent::No
    }

    fn shutdown(&mut self) {}

    fn is_shutdown(&self) -> bool {
        true
    }
}
