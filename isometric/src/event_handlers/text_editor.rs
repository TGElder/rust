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

    fn handle_key(&mut self, keycode: glutin::VirtualKeyCode, shift_state: bool) {
        match keycode {
            glutin::VirtualKeyCode::A => self.insert_letter('A', shift_state),
            glutin::VirtualKeyCode::B => self.insert_letter('B', shift_state),
            glutin::VirtualKeyCode::C => self.insert_letter('C', shift_state),
            glutin::VirtualKeyCode::D => self.insert_letter('D', shift_state),
            glutin::VirtualKeyCode::E => self.insert_letter('E', shift_state),
            glutin::VirtualKeyCode::F => self.insert_letter('F', shift_state),
            glutin::VirtualKeyCode::G => self.insert_letter('G', shift_state),
            glutin::VirtualKeyCode::H => self.insert_letter('H', shift_state),
            glutin::VirtualKeyCode::I => self.insert_letter('I', shift_state),
            glutin::VirtualKeyCode::J => self.insert_letter('J', shift_state),
            glutin::VirtualKeyCode::K => self.insert_letter('K', shift_state),
            glutin::VirtualKeyCode::L => self.insert_letter('L', shift_state),
            glutin::VirtualKeyCode::M => self.insert_letter('M', shift_state),
            glutin::VirtualKeyCode::N => self.insert_letter('N', shift_state),
            glutin::VirtualKeyCode::O => self.insert_letter('O', shift_state),
            glutin::VirtualKeyCode::P => self.insert_letter('P', shift_state),
            glutin::VirtualKeyCode::Q => self.insert_letter('Q', shift_state),
            glutin::VirtualKeyCode::R => self.insert_letter('R', shift_state),
            glutin::VirtualKeyCode::S => self.insert_letter('S', shift_state),
            glutin::VirtualKeyCode::T => self.insert_letter('T', shift_state),
            glutin::VirtualKeyCode::U => self.insert_letter('U', shift_state),
            glutin::VirtualKeyCode::V => self.insert_letter('V', shift_state),
            glutin::VirtualKeyCode::W => self.insert_letter('W', shift_state),
            glutin::VirtualKeyCode::X => self.insert_letter('X', shift_state),
            glutin::VirtualKeyCode::Y => self.insert_letter('Y', shift_state),
            glutin::VirtualKeyCode::Z => self.insert_letter('Z', shift_state),
            glutin::VirtualKeyCode::Space => self.insert_character(' '),
            glutin::VirtualKeyCode::Key0 => self.insert_character('0'),
            glutin::VirtualKeyCode::Key1 => self.insert_character('1'),
            glutin::VirtualKeyCode::Key2 => self.insert_character('2'),
            glutin::VirtualKeyCode::Key3 => self.insert_character('3'),
            glutin::VirtualKeyCode::Key4 => self.insert_character('4'),
            glutin::VirtualKeyCode::Key5 => self.insert_character('5'),
            glutin::VirtualKeyCode::Key6 => self.insert_character('6'),
            glutin::VirtualKeyCode::Key7 => self.insert_character('7'),
            glutin::VirtualKeyCode::Key8 => self.insert_character('8'),
            glutin::VirtualKeyCode::Key9 => self.insert_character('9'),
            glutin::VirtualKeyCode::Add => self.insert_character('+'),
            glutin::VirtualKeyCode::Apostrophe => self.insert_character('\''),
            glutin::VirtualKeyCode::Backslash => self.insert_character('\\'),
            glutin::VirtualKeyCode::Comma => self.insert_character(','),
            glutin::VirtualKeyCode::Colon => self.insert_character(':'),
            glutin::VirtualKeyCode::Equals => self.insert_character('='),
            glutin::VirtualKeyCode::Period => self.insert_character('.'),
            glutin::VirtualKeyCode::Semicolon => self.insert_character(';'),
            glutin::VirtualKeyCode::Slash => self.insert_character('/'),
            glutin::VirtualKeyCode::Subtract => self.insert_character('-'),
            glutin::VirtualKeyCode::Back => self.backspace(),
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
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event:
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(key),
                                state: glutin::ElementState::Pressed,
                                modifiers:
                                    glutin::ModifiersState {
                                        shift: shift_state, ..
                                    },
                                ..
                            },
                        ..
                    },
                ..
            }) => {
                self.handle_key(key, shift_state);
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

        text_editor.handle_key(glutin::VirtualKeyCode::A, false);

        assert_eq!(text_editor.text(), "a");
    }

    #[test]
    fn test_uppercase_character() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::VirtualKeyCode::A, true);

        assert_eq!(text_editor.text(), "A");
    }

    #[test]
    fn test_word() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::VirtualKeyCode::H, true);
        text_editor.handle_key(glutin::VirtualKeyCode::E, false);
        text_editor.handle_key(glutin::VirtualKeyCode::L, false);
        text_editor.handle_key(glutin::VirtualKeyCode::L, false);
        text_editor.handle_key(glutin::VirtualKeyCode::O, false);

        assert_eq!(text_editor.text(), "Hello");
    }

    #[test]
    fn test_backspace_2_chars() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::VirtualKeyCode::H, true);
        text_editor.handle_key(glutin::VirtualKeyCode::E, false);
        text_editor.handle_key(glutin::VirtualKeyCode::Back, false);

        assert_eq!(text_editor.text(), "H");
    }

    #[test]
    fn test_backspace_1_char() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::VirtualKeyCode::H, true);
        text_editor.handle_key(glutin::VirtualKeyCode::Back, false);

        assert_eq!(text_editor.text(), "");
    }

    #[test]
    fn test_backspace_0_chars() {
        let mut text_editor = TextEditor::default();

        text_editor.handle_key(glutin::VirtualKeyCode::Back, false);

        assert_eq!(text_editor.text(), "");
    }

}
