use crate::label_editor::*;
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::WithWorld;
use commons::async_channel::Sender;
use commons::async_trait::async_trait;
use commons::bincode::{deserialize_from, serialize_into};
use commons::grid::Grid;
use commons::V2;
use isometric::EventHandler;
use isometric::{coords::*, Command, Event};
use isometric::{Button, ElementState, VirtualKeyCode};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;

pub struct Labels<T> {
    tx: T,
    command_tx: Sender<Vec<Command>>,
    label_editor: LabelEditor,
    world_coord: Option<WorldCoord>,
    binding: Button,
}

impl<T> Labels<T>
where
    T: WithWorld,
{
    pub fn new(tx: T, command_tx: Sender<Vec<Command>>) -> Labels<T> {
        Labels {
            tx,
            command_tx,
            label_editor: LabelEditor::new(HashMap::new()),
            world_coord: None,
            binding: Button::Key(VirtualKeyCode::L),
        }
    }

    pub async fn init(&self) {
        let commands = self.label_editor.draw_all();
        self.command_tx.send(commands).await.unwrap();
    }

    async fn start_edit(&mut self) {
        let world_coord = unwrap_or!(self.world_coord, return);
        let position = world_coord.to_v2_round();
        let z = unwrap_or!(self.get_elevation(&position).await, return);
        self.label_editor
            .start_edit(WorldCoord::new(position.x as f32, position.y as f32, z));
    }

    async fn get_elevation(&mut self, position: &V2<usize>) -> Option<f32> {
        self.tx
            .with_world(|world| {
                world
                    .get_cell(position)
                    .map(|cell| cell.elevation.max(world.sea_level()))
            })
            .await
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    pub fn save(&self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        serialize_into(&mut file, &self.label_editor.labels()).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        let labels = deserialize_from(file).unwrap();
        self.label_editor = LabelEditor::new(labels);
    }

    fn get_path(path: &str) -> String {
        format!("{}.labels", path)
    }
}

#[async_trait]
impl<T> HandleEngineEvent for Labels<T>
where
    T: WithWorld + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        let editor_commands = self.label_editor.handle_event(event.clone());
        if !editor_commands.is_empty() {
            self.command_tx.send(editor_commands).await.unwrap();
            return capture_if_keypress(event);
        }
        match *event {
            Event::WorldPositionChanged(world_coord) => self.update_world_coord(world_coord),
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers,
            } if button == &self.binding && !modifiers.alt() => self.start_edit().await,
            _ => (),
        }
        Capture::No
    }
}

fn capture_if_keypress(event: Arc<Event>) -> Capture {
    if let Event::Button {
        button: Button::Key(..),
        ..
    } = *event
    {
        Capture::Yes
    } else {
        Capture::No
    }
}
