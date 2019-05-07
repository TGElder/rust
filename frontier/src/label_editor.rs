use isometric::coords::WorldCoord;
use isometric::drawing::Text;
use isometric::event_handlers::TextEditor;
use isometric::EventHandler;
use isometric::Font;
use isometric::Texture;
use isometric::{Command, Event};
use isometric::{ElementState, VirtualKeyCode};

use std::sync::Arc;

pub struct LabelEditor {
    font: Arc<Font>,
    edit: Option<LabelEdit>,
}

impl LabelEditor {
    pub fn new() -> LabelEditor {
        LabelEditor {
            font: Arc::new(Font::from_csv_and_texture(
                "serif.csv",
                Texture::new(image::open("serif.png").unwrap()),
            )),
            edit: None,
        }
    }

    pub fn start_edit(&mut self, world_coord: WorldCoord) {
        self.edit = Some(LabelEdit::new(self.font.clone(), world_coord));
    }
}

impl EventHandler for LabelEditor {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        if let Some(edit) = &mut self.edit {
            match *event {
                Event::Key {
                    key: VirtualKeyCode::Return,
                    state: ElementState::Pressed,
                    ..
                } => {
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
            text_editor: TextEditor::new(),
        }
    }
}

impl EventHandler for LabelEdit {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        self.text_editor.handle_event(event.clone());
        let name = format!("{:?}", self.world_coord);
        vec![Command::Draw {
            name,
            drawing: Box::new(Text::new(
                &self.text_editor.text(),
                self.world_coord,
                self.font.clone(),
            )),
        }]
    }
}
