use super::*;
use isometric::{Button, ElementState, VirtualKeyCode};

pub struct SelectAvatar {
    command_tx: Sender<GameCommand>,
}

impl SelectAvatar {
    pub fn new(command_tx: Sender<GameCommand>) -> SelectAvatar {
        SelectAvatar { command_tx }
    }

    fn select_avatar(&mut self, name: String) {
        self.command_tx
            .send(GameCommand::SelectAvatar(name))
            .unwrap();
    }
}

impl GameEventConsumer for SelectAvatar {
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
