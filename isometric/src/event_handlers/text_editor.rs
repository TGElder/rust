use engine::{Command, Event};
use events::EventHandler;
use std::default::Default;
use std::sync::Arc;

pub struct TextEditor {
    caret_position: usize,
    text: String,
}

impl TextEditor {
    pub fn text(&self) -> &str {
        &self.text
    }

    fn insert_letter(&mut self, mut character: char, uppercase: bool) {
        if !uppercase {
            character = character.to_ascii_lowercase();
        }
        self.insert_character(character);
    }

    fn insert_character(&mut self, character: char) {
        self.text.insert(self.caret_position, character);
        self.caret_position += 1;
    }

    fn backspace(&mut self) {
        if self.caret_position > 0 {
            self.text.remove(self.caret_position - 1);
            self.caret_position -= 1;
        }
    }

    fn handle_key(&mut self, keycode: glutin::event::VirtualKeyCode, shift_state: bool) {
        match keycode {
            glutin::event::VirtualKeyCode::A => self.insert_letter('A', shift_state),
            glutin::event::VirtualKeyCode::B => self.insert_letter('B', shift_state),
            glutin::event::VirtualKeyCode::C => self.insert_letter('C', shift_state),
            glutin::event::VirtualKeyCode::D => self.insert_letter('D', shift_state),
            glutin::event::VirtualKeyCode::E => self.insert_letter('E', shift_state),
            glutin::event::VirtualKeyCode::F => self.insert_letter('F', shift_state),
            glutin::event::VirtualKeyCode::G => self.insert_letter('G', shift_state),
            glutin::event::VirtualKeyCode::H => self.insert_letter('H', shift_state),
            glutin::event::VirtualKeyCode::I => self.insert_letter('I', shift_state),
            glutin::event::VirtualKeyCode::J => self.insert_letter('J', shift_state),
            glutin::event::VirtualKeyCode::K => self.insert_letter('K', shift_state),
            glutin::event::VirtualKeyCode::L => self.insert_letter('L', shift_state),
            glutin::event::VirtualKeyCode::M => self.insert_letter('M', shift_state),
            glutin::event::VirtualKeyCode::N => self.insert_letter('N', shift_state),
            glutin::event::VirtualKeyCode::O => self.insert_letter('O', shift_state),
            glutin::event::VirtualKeyCode::P => self.insert_letter('P', shift_state),
            glutin::event::VirtualKeyCode::Q => self.insert_letter('Q', shift_state),
            glutin::event::VirtualKeyCode::R => self.insert_letter('R', shift_state),
            glutin::event::VirtualKeyCode::S => self.insert_letter('S', shift_state),
            glutin::event::VirtualKeyCode::T => self.insert_letter('T', shift_state),
            glutin::event::VirtualKeyCode::U => self.insert_letter('U', shift_state),
            glutin::event::VirtualKeyCode::V => self.insert_letter('V', shift_state),
            glutin::event::VirtualKeyCode::W => self.insert_letter('W', shift_state),
            glutin::event::VirtualKeyCode::X => self.insert_letter('X', shift_state),
            glutin::event::VirtualKeyCode::Y => self.insert_letter('Y', shift_state),
            glutin::event::VirtualKeyCode::Z => self.insert_letter('Z', shift_state),
            glutin::event::VirtualKeyCode::Space => self.insert_character(' '),
            glutin::event::VirtualKeyCode::Key0 => self.insert_character('0'),
            glutin::event::VirtualKeyCode::Key1 => self.insert_character('1'),
            glutin::event::VirtualKeyCode::Key2 => self.insert_character('2'),
            glutin::event::VirtualKeyCode::Key3 => self.insert_character('3'),
            glutin::event::VirtualKeyCode::Key4 => self.insert_character('4'),
            glutin::event::VirtualKeyCode::Key5 => self.insert_character('5'),
            glutin::event::VirtualKeyCode::Key6 => self.insert_character('6'),
            glutin::event::VirtualKeyCode::Key7 => self.insert_character('7'),
            glutin::event::VirtualKeyCode::Key8 => self.insert_character('8'),
            glutin::event::VirtualKeyCode::Key9 => self.insert_character('9'),
            glutin::event::VirtualKeyCode::Minus => self.insert_character('+'),
            glutin::event::VirtualKeyCode::Apostrophe => self.insert_character('\''),
            glutin::event::VirtualKeyCode::Backslash => self.insert_character('\\'),
            glutin::event::VirtualKeyCode::Comma => self.insert_character(','),
            glutin::event::VirtualKeyCode::Colon => self.insert_character(':'),
            glutin::event::VirtualKeyCode::Equals => self.insert_character('='),
            glutin::event::VirtualKeyCode::Period => self.insert_character('.'),
            glutin::event::VirtualKeyCode::Semicolon => self.insert_character(';'),
            glutin::event::VirtualKeyCode::Slash => self.insert_character('/'),
            glutin::event::VirtualKeyCode::Plus => self.insert_character('-'),
            glutin::event::VirtualKeyCode::Back => self.backspace(),
            _ => {}
        }
    }
}

impl Default for TextEditor {
    fn default() -> TextEditor {
        TextEditor {
            caret_position: 0,
            text: String::new(),
        }
    }
}

impl EventHandler for TextEditor {

    #[allow(deprecated)]
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::event::Event::WindowEvent {
                event:
                    glutin::event::WindowEvent::KeyboardInput {
                        input:
                            glutin::event::KeyboardInput {
                                virtual_keycode: Some(key),
                                state: glutin::event::ElementState::Pressed,
                                modifiers,
                                ..
                            },
                        ..
                    },
                ..
            }) => {
                self.handle_key(key, modifiers == glutin::event::ModifiersState::SHIFT);
                vec![]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_lowercase_character() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::event::VirtualKeyCode::A, false);

        assert_eq!(text_editor.text(), "a");
    }

    #[test]
    fn test_uppercase_character() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::event::VirtualKeyCode::A, true);

        assert_eq!(text_editor.text(), "A");
    }

    #[test]
    fn test_word() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::event::VirtualKeyCode::H, true);
        text_editor.handle_key(glutin::event::VirtualKeyCode::E, false);
        text_editor.handle_key(glutin::event::VirtualKeyCode::L, false);
        text_editor.handle_key(glutin::event::VirtualKeyCode::L, false);
        text_editor.handle_key(glutin::event::VirtualKeyCode::O, false);

        assert_eq!(text_editor.text(), "Hello");
    }

    #[test]
    fn test_backspace_2_chars() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::event::VirtualKeyCode::H, true);
        text_editor.handle_key(glutin::event::VirtualKeyCode::E, false);
        text_editor.handle_key(glutin::event::VirtualKeyCode::Back, false);

        assert_eq!(text_editor.text(), "H");
    }

    #[test]
    fn test_backspace_1_char() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::event::VirtualKeyCode::H, true);
        text_editor.handle_key(glutin::event::VirtualKeyCode::Back, false);

        assert_eq!(text_editor.text(), "");
    }

    #[test]
    fn test_backspace_0_chars() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::event::VirtualKeyCode::Back, false);

        assert_eq!(text_editor.text(), "");
    }
}
