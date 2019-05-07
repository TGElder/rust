use crate::avatar::*;
use crate::house_builder::*;
use crate::label_editor::*;
use crate::world::*;
use crate::world_artist::*;

use isometric::coords::*;
use isometric::terrain::*;
use isometric::EventHandler;
use isometric::{v2, V3};
use isometric::{Command, Event};
use isometric::{ElementState, VirtualKeyCode};

use std::f32::consts::PI;
use std::sync::Arc;

pub struct GameHandler {
    world: World,
    world_artist: WorldArtist,
    world_coord: Option<WorldCoord>,
    label_editor: LabelEditor,
    house_builder: HouseBuilder,
    avatar: Avatar,
}

impl GameHandler {
    pub fn new(world: World) -> GameHandler {
        let cliff_gradient = 0.53;
        let beach_level = world.sea_level() + 0.05;
        let light_direction = V3::new(-1.0, 0.0, 1.0);
        let world_artist =
            WorldArtist::new(&world, 64, cliff_gradient, beach_level, light_direction);
        GameHandler {
            house_builder: HouseBuilder::new(world.width(), world.height(), light_direction),
            world,
            world_artist,
            world_coord: None,
            label_editor: LabelEditor::new(),
            avatar: Avatar::new(0.00078125, cliff_gradient),
        }
    }
}

impl GameHandler {
    fn build_road(&mut self) -> Vec<Command> {
        let from = self.avatar.position();
        self.avatar.walk(&self.world);
        let to = self.avatar.position();
        match (from, to) {
            (Some(from), Some(to)) if from != to => {
                let from = v2(from.x as usize, from.y as usize);
                let to = v2(to.x as usize, to.y as usize);

                let edge = Edge::new(from, to);
                self.world.toggle_road(&edge);
                let mut commands = self.world_artist.draw_affected(&self.world, vec![from, to]);
                commands.append(&mut self.avatar.draw());
                commands
            }
            _ => vec![],
        }
    }

    fn rotate(&self, yaw: f32) -> Vec<Command> {
        let mut commands = vec![Command::Rotate {
            center: GLCoord4D::new(0.0, 0.0, 0.0, 1.0),
            yaw,
        }];
        commands.append(&mut self.avatar.draw());
        commands
    }

    fn build_house(&mut self) -> Vec<Command> {
        if let Some(world_coord) = self.world_coord {
            let world_coord = self.world.snap_middle(world_coord);
            self.house_builder.build_house(world_coord)
        } else {
            vec![]
        }
    }
}

impl EventHandler for GameHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        let label_commands = self.label_editor.handle_event(event.clone());
        if !label_commands.is_empty() {
            label_commands
        } else {
            match *event {
                Event::Start => self.world_artist.init(&self.world),
                Event::WorldPositionChanged(world_coord) => {
                    self.world_coord = Some(world_coord);
                    vec![]
                }
                Event::Key {
                    key,
                    state: ElementState::Pressed,
                    ..
                } => match key {
                    VirtualKeyCode::H => {
                        self.avatar.reposition(self.world_coord, &self.world);
                        self.avatar.draw()
                    }
                    VirtualKeyCode::W => {
                        self.avatar.walk(&self.world);
                        self.avatar.draw()
                    }
                    VirtualKeyCode::A => {
                        self.avatar.rotate_anticlockwise();
                        self.avatar.draw()
                    }
                    VirtualKeyCode::D => {
                        self.avatar.rotate_clockwise();
                        self.avatar.draw()
                    }
                    VirtualKeyCode::Q => self.rotate(PI / 16.0),
                    VirtualKeyCode::E => self.rotate(-PI / 16.0),
                    VirtualKeyCode::R => self.build_road(),
                    VirtualKeyCode::L => {
                        if let Some(world_coord) = self.avatar.position() {
                            self.label_editor.start_edit(world_coord);
                        }
                        vec![]
                    }
                    VirtualKeyCode::B => self.build_house(),
                    _ => vec![],
                },
                _ => vec![],
            }
        }
    }
}
