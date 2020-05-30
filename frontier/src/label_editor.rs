use isometric::coords::WorldCoord;
use isometric::drawing::draw_label;
use isometric::event_handlers::TextEditor;
use isometric::EventHandler;
use isometric::Font;
use isometric::{Button, Command, Event};
use isometric::{ElementState, VirtualKeyCode};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub struct LabelEditor {
    font: Arc<Font>,
    edit: Option<LabelEdit>,
    labels: HashMap<String, Label>,
}

impl LabelEditor {
    pub fn new(labels: HashMap<String, Label>) -> LabelEditor {
        LabelEditor {
            font: Arc::new(Font::from_csv_and_texture("serif.csv", "serif.png")),
            edit: None,
            labels,
        }
    }

    pub fn start_edit(&mut self, world_coord: WorldCoord) {
        self.edit = Some(LabelEdit::new(self.font.clone(), world_coord));
    }

    pub fn labels(&self) -> &HashMap<String, Label> {
        &self.labels
    }

    pub fn draw_all(&self) -> Vec<Command> {
        let mut out = vec![];
        for label in self.labels.values() {
            out.append(&mut label.draw(&self.font));
        }
        out
    }
}

impl EventHandler for LabelEditor {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        if let Some(edit) = &mut self.edit {
            match *event {
                Event::Button {
                    button: Button::Key(VirtualKeyCode::Return),
                    state: ElementState::Pressed,
                    ..
                } => {
                    let label = edit.label();
                    self.labels.insert(label.name.clone(), label);
                    self.edit = None;
                    vec![]
                }
                _ => edit.handle_event(event),
            }
        } else {
            vec![]
        }
    }
}

struct LabelEdit {
    font: Arc<Font>,
    world_coord: WorldCoord,
    text_editor: TextEditor,
}

impl LabelEdit {
    pub fn new(font: Arc<Font>, world_coord: WorldCoord) -> LabelEdit {
        LabelEdit {
            world_coord,
            font,
            text_editor: TextEditor::default(),
        }
    }

    fn label(&self) -> Label {
        Label::new(self.world_coord, self.text_editor.text().to_string())
    }

    fn draw(&self) -> Vec<Command> {
        self.label().draw(&self.font)
    }
}

impl EventHandler for LabelEdit {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        self.text_editor.handle_event(event);
        self.draw()
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Label {
    name: String,
    world_coord: WorldCoord,
    text: String,
}

impl Label {
    fn new(world_coord: WorldCoord, text: String) -> Label {
        Label {
            name: format!("{:?}", world_coord),
            world_coord,
            text,
        }
    }

    fn draw(&self, font: &Font) -> Vec<Command> {
        let name = format!("{:?}", self.world_coord);
        draw_label(name, &self.text, self.world_coord, font)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_label() {
        let original = Label::new(WorldCoord::new(0.1, 2.0, 30.0), "test".to_string());
        let encoded: Vec<u8> = bincode::serialize(&original).unwrap();
        let reconstructed: Label = bincode::deserialize(&encoded[..]).unwrap();
        assert_eq!(original, reconstructed);
    }
}
