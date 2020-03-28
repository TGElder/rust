use super::*;
use crate::label_editor::*;
use isometric::coords::*;
use isometric::EventHandler;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};

const HANDLE: &str = "label_editor_handler";

pub struct LabelEditorHandler {
    game_tx: UpdateSender<Game>,
    label_editor: LabelEditor,
    world_coord: Option<WorldCoord>,
    binding: Button,
}

impl LabelEditorHandler {
    pub fn new(game_tx: &UpdateSender<Game>) -> LabelEditorHandler {
        LabelEditorHandler {
            game_tx: game_tx.clone_with_handle(HANDLE),
            label_editor: LabelEditor::new(HashMap::new()),
            world_coord: None,
            binding: Button::Key(VirtualKeyCode::L),
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn start_edit(&mut self, game_state: &GameState) {
        if let Some(world_coord) = self.world_coord {
            if let Some(cell) = game_state.world.get_cell(&world_coord.to_v2_round()) {
                let z = game_state.world.sea_level().max(cell.elevation);
                self.label_editor
                    .start_edit(WorldCoord { z, ..world_coord });
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
        self.game_tx
            .update(move |game| game.send_engine_commands(commands));
    }
}

impl GameEventConsumer for LabelEditorHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

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
            self.game_tx
                .update(move |game| game.send_engine_commands(editor_commands));
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

    fn shutdown(&mut self) {}

    fn is_shutdown(&self) -> bool {
        true
    }
}