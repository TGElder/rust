use super::*;
use isometric::{Button, ElementState, VirtualKeyCode};

const NAME: &str = "select_avatar";

pub struct SelectAvatar {
    game_tx: FnSender<Game>,
}

impl SelectAvatar {
    pub fn new(game_tx: &FnSender<Game>) -> SelectAvatar {
        SelectAvatar {
            game_tx: game_tx.clone_with_name(NAME),
        }
    }

    fn select_avatar(&mut self, name: String) {
        self.game_tx.send(move |game: &mut Game| {
            game.mut_state().selected_avatar = Some(name);
        });
    }
}

impl GameEventConsumer for SelectAvatar {
    fn name(&self) -> &'static str {
        NAME
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
            if *button == Button::Key(VirtualKeyCode::Key0) {
                self.select_avatar(String::from("0"));
            } else if *button == Button::Key(VirtualKeyCode::Key1) {
                self.select_avatar(String::from("1"));
            } else if *button == Button::Key(VirtualKeyCode::Key2) {
                self.select_avatar(String::from("2"));
            } else if *button == Button::Key(VirtualKeyCode::Key3) {
                self.select_avatar(String::from("3"));
            } else if *button == Button::Key(VirtualKeyCode::Key4) {
                self.select_avatar(String::from("4"));
            } else if *button == Button::Key(VirtualKeyCode::Key5) {
                self.select_avatar(String::from("5"));
            } else if *button == Button::Key(VirtualKeyCode::Key6) {
                self.select_avatar(String::from("6"));
            } else if *button == Button::Key(VirtualKeyCode::Key7) {
                self.select_avatar(String::from("7"));
            } else if *button == Button::Key(VirtualKeyCode::Key8) {
                self.select_avatar(String::from("8"));
            } else if *button == Button::Key(VirtualKeyCode::Key9) {
                self.select_avatar(String::from("9"));
            }
        }
        CaptureEvent::No
    }
}
