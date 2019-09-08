use super::*;
use crate::label_editor::*;
use commons::*;
use isometric::coords::*;
use isometric::EventHandler;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub struct LabelEditorHandler {
    command_tx: Sender<GameCommand>,
    label_editor: LabelEditor,
    world_coord: Option<WorldCoord>,
    binding: Button,
}

impl LabelEditorHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> LabelEditorHandler {
        LabelEditorHandler {
            command_tx,
            label_editor: LabelEditor::new(HashMap::new()),
            world_coord: None,
            binding: Button::Key(VirtualKeyCode::L),
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn start_edit(&mut self, game_state: &GameState) {
        if let Some(WorldCoord { x, y, .. }) = self.world_coord {
            let x = x.round() as usize;
            let y = y.round() as usize;
            if let Some(cell) = game_state.world.get_cell(&v2(x, y)) {
                self.label_editor
                    .start_edit(WorldCoord::new(x as f32, y as f32, cell.elevation));
            }
        }
    }

    fn get_path(path: &str) -> String {
        format!("{}.labels", path)
    }

    fn save(&mut self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.label_editor.labels()).unwrap();
    }

    fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        let labels = bincode::deserialize_from(file).unwrap();
        self.label_editor = LabelEditor::new(labels);
        let commands = self.label_editor.draw_all();
        self.command_tx
            .send(GameCommand::EngineCommands(commands))
            .unwrap();
    }
}

impl GameEventConsumer for LabelEditorHandler {
    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Save(path) => self.save(&path),
            GameEvent::Load(path) => self.load(&path),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        }
        let editor_commands = self.label_editor.handle_event(event.clone());
        if !editor_commands.is_empty() {
            self.command_tx
                .send(GameCommand::EngineCommands(editor_commands))
                .unwrap();
            return CaptureEvent::Yes;
        }
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                self.start_edit(&game_state)
            }
        }
        CaptureEvent::No
    }
}
