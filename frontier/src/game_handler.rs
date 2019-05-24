use crate::avatar::*;
use crate::house_builder::*;
use crate::label_editor::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use crate::shore_start::*;
use crate::world::*;
use crate::world_artist::*;

use commons::{v2, V3};
use isometric::coords::*;
use isometric::event_handlers::{RotateHandler, ZoomHandler};
use isometric::EventHandler;
use isometric::{Command, Event};
use isometric::{ElementState, MouseButton, VirtualKeyCode};

use rand::prelude::*;
use std::sync::Arc;
use std::time::Instant;

pub struct GameHandler {
    world: World,
    world_artist: WorldArtist,
    mouse_coord: Option<WorldCoord>,
    label_editor: LabelEditor,
    house_builder: HouseBuilder,
    avatar: Avatar,
    avatar_artist: AvatarArtist,
    avatar_pathfinder: Pathfinder,
    follow_avatar: bool,
    road_builder: RoadBuilder,
    handlers: Vec<Box<EventHandler>>,
}

impl GameHandler {
    pub fn new(world: World) -> GameHandler {
        let beach_level = world.sea_level() + 0.05;
        let snow_level = world.max_height() * 0.8;
        let cliff_gradient = 0.5;
        let light_direction = V3::new(-1.0, 0.0, 1.0);
        let world_artist = WorldArtist::new(
            &world,
            64,
            beach_level,
            snow_level,
            cliff_gradient,
            light_direction,
        );
        let avatar = Avatar::new(0.1);
        GameHandler {
            house_builder: HouseBuilder::new(world.width(), world.height(), light_direction),
            avatar_pathfinder: Pathfinder::new(&world, avatar.travel_duration()),
            avatar,
            road_builder: RoadBuilder::new(&world),
            world,
            world_artist,
            mouse_coord: None,
            label_editor: LabelEditor::new(),
            avatar_artist: AvatarArtist::new(0.00078125, light_direction),
            follow_avatar: true,
            handlers: vec![
                Box::new(ZoomHandler::new()),
                Box::new(RotateHandler::new(VirtualKeyCode::Q, VirtualKeyCode::E)),
            ],
        }
    }
}

impl GameHandler {
    fn handle_road_builder_result(&mut self, result: Option<RoadBuilderResult>) -> Vec<Command> {
        result
            .map(|result| {
                result.update_pathfinder(&self.world, &mut self.avatar_pathfinder);
                self.world_artist.draw_affected(&self.world, result.path())
            })
            .unwrap_or(vec![])
    }

    fn auto_build_road(&mut self) -> Vec<Command> {
        if let Some(WorldCoord { x, y, .. }) = self.mouse_coord {
            let to = v2(x.round() as usize, y.round() as usize);
            let result = self
                .road_builder
                .auto_build_road(&mut self.world, &self.avatar, &to);
            return self.handle_road_builder_result(result);
        }
        return vec![];
    }

    fn build_road(&mut self) -> Vec<Command> {
        let result = self
            .road_builder
            .build_forward(&mut self.world, &self.avatar);
        result.iter().for_each(|_| {
            self.avatar
                .walk_forward(&self.world, &self.avatar_pathfinder)
        });
        let mut commands = self.handle_road_builder_result(result);
        commands.append(&mut self.avatar_artist.draw(&self.avatar, &self.world));
        commands
    }

    fn build_house(&mut self) -> Vec<Command> {
        if let Some(mouse_coord) = self.mouse_coord {
            let mouse_coord = self.world.snap_to_middle(mouse_coord);
            self.house_builder.build_house(mouse_coord)
        } else {
            vec![]
        }
    }

    fn walk_to(&mut self) {
        if let Some(WorldCoord { x, y, .. }) = self.mouse_coord {
            let to = v2(x.round() as usize, y.round() as usize);
            self.avatar
                .walk_to(&self.world, &to, &self.avatar_pathfinder);
        }
    }

    fn center(&self) -> Command {
        let x = self.world.width() / 2;
        let y = self.world.width() / 2;
        let z = self.world.get_elevation(&v2(x, y)).unwrap();
        Command::LookAt(WorldCoord::new(x as f32, y as f32, z))
    }

    fn shore_start(&mut self) {
        let shore_start = shore_start(32, &self.world, &mut Box::new(SmallRng::from_entropy()));
        self.avatar.reposition(shore_start.at(), Rotation::Up);
        self.avatar.walk_to(
            &self.world,
            &shore_start.landfall(),
            &self.avatar_pathfinder,
        );
    }
}

impl EventHandler for GameHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        self.world.set_time(Instant::now());
        self.avatar.evolve(&self.world);
        let label_commands = self.label_editor.handle_event(event.clone());
        if !label_commands.is_empty() {
            label_commands
        } else {
            let mut commands = vec![];
            match *event {
                Event::Start => {
                    self.shore_start();
                    commands.append(&mut self.world_artist.init(&self.world));
                    commands.push(self.center());
                }
                Event::WorldPositionChanged(mouse_coord) => {
                    self.mouse_coord = Some(mouse_coord);
                }
                Event::Key {
                    key,
                    state: ElementState::Pressed,
                    ..
                } => match key {
                    VirtualKeyCode::H => {
                        if let Some(WorldCoord { x, y, .. }) = self.mouse_coord {
                            self.avatar.reposition(
                                v2(x.round() as usize, y.round() as usize),
                                Rotation::Down,
                            );
                        };
                    }
                    VirtualKeyCode::W => {
                        self.avatar
                            .walk_forward(&self.world, &self.avatar_pathfinder);
                    }
                    VirtualKeyCode::A => {
                        self.avatar.rotate_anticlockwise();
                    }
                    VirtualKeyCode::D => {
                        self.avatar.rotate_clockwise();
                    }
                    VirtualKeyCode::R => commands.append(&mut self.build_road()),
                    VirtualKeyCode::X => commands.append(&mut self.auto_build_road()),
                    VirtualKeyCode::L => {
                        if let Some(AvatarState::Stationary { .. }) = self.avatar.state() {
                            self.label_editor
                                .start_edit(self.avatar.compute_world_coord(&self.world).unwrap());
                        }
                    }
                    VirtualKeyCode::B => commands.append(&mut self.build_house()),
                    VirtualKeyCode::C => self.follow_avatar = !self.follow_avatar,
                    _ => (),
                },
                Event::Mouse {
                    state: ElementState::Pressed,
                    button: MouseButton::Right,
                } => self.walk_to(),
                _ => (),
            };
            for handler in self.handlers.iter_mut() {
                commands.append(&mut handler.handle_event(event.clone()));
            }
            if self.follow_avatar {
                if let Some(world_coord) = self.avatar.compute_world_coord(&self.world) {
                    commands.push(Command::LookAt(world_coord));
                }
            }
            self.world.set_time(Instant::now());
            commands.append(&mut self.avatar_artist.draw(&self.avatar, &self.world));
            commands
        }
    }
}
