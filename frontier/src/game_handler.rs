use crate::avatar::*;
use crate::house_builder::*;
use crate::label_editor::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use crate::shore_start::*;
use crate::visibility_computer::*;
use crate::world::*;
use crate::world_gen::*;

use commons::*;
use isometric::cell_traits::*;
use isometric::coords::*;
use isometric::drawing::*;
use isometric::event_handlers::{RotateHandler, ZoomHandler};
use isometric::Color;
use isometric::EventHandler;
use isometric::{Command, Event};
use isometric::{ElementState, ModifiersState, MouseButton, VirtualKeyCode};

use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use std::time::Instant;

pub struct GameHandler {
    world: World,
    world_gen_params: WorldGenParameters,
    clock: Instant,
    visibility_computer: VisibilityComputer,
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
    rotate_handler: RotateHandler,
}

impl GameHandler {
    pub fn new<T: Rng>(
        world: World,
        world_gen_params: WorldGenParameters,
        rng: &mut T,
    ) -> GameHandler {
        let visibility_computer = VisibilityComputer::new(0.002, Some(6371.0));
        let shore_start = shore_start(32, &world, rng);
        let houses = M::from_element(world.width(), world.height(), false);
        GameHandler::load(Load {
            world,
            world_gen_params,
            visibility_computer,
            avatar_position: shore_start.at(),
            avatar_rotation: shore_start.rotation(),
            labels: HashMap::new(),
            houses,
        })
    }

    pub fn load(load: Load) -> GameHandler {
        let world = load.world;
        let light_direction = V3::new(-1.0, 0.0, 1.0);
        let world_artist = WorldArtist::new(
            &world,
            Self::create_coloring(
                &world,
                light_direction,
                load.world_gen_params.cliff_gradient,
            ),
            64,
        );
        let mut avatar = Avatar::new(0.1);
        avatar.reposition(load.avatar_position, load.avatar_rotation);
        GameHandler {
            world_gen_params: load.world_gen_params,
            clock: Instant::now(),
            house_builder: HouseBuilder::new(load.houses, light_direction),
            avatar_pathfinder: Pathfinder::new(&world, avatar.travel_duration()),
            avatar,
            road_builder: RoadBuilder::new(&world),
            visibility_computer: load.visibility_computer,
            world,
            world_artist,
            mouse_coord: None,
            label_editor: LabelEditor::new(load.labels),
            avatar_artist: AvatarArtist::new(0.000_781_25, light_direction),
            follow_avatar: true,
            handlers: vec![Box::new(ZoomHandler::default())],
            rotate_handler: RotateHandler::new(VirtualKeyCode::Q, VirtualKeyCode::E),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn create_coloring(
        world: &World,
        light_direction: V3<f32>,
        cliff_gradient: f32,
    ) -> LayerColoring<WorldCell> {
        let beach_level = world.sea_level() + 0.05;
        let snow_temperature = 0.0;
        let mut out = LayerColoring::default();
        out.add_layer(
            "base".to_string(),
            Box::new(DefaultColoring::new(
                &world,
                beach_level,
                snow_temperature,
                cliff_gradient,
                light_direction,
            )),
            0,
        );
        out
    }

    pub fn set_overlay<T>(&mut self, overlay: T)
    where
        T: 'static + TerrainColoring<WorldCell> + Send,
    {
        self.world_artist
            .coloring()
            .add_layer("overlay".to_string(), Box::new(overlay), 1);
    }

    pub fn toggle_overlay(&mut self) -> Vec<Command> {
        if let Some(priority) = self.world_artist.coloring().get_priority("overlay") {
            self.world_artist
                .coloring()
                .set_priority("overlay", -priority);
        }
        self.world_artist.init(&self.world)
    }

    fn handle_road_builder_result(&mut self, result: Option<RoadBuilderResult>) -> Vec<Command> {
        result
            .map(|result| {
                result.update_pathfinder(&self.world, &mut self.avatar_pathfinder);
                self.world_artist.draw_affected(&self.world, result.path())
            })
            .unwrap_or_else(|| vec![])
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
                .walk_forward(&self.world, &self.avatar_pathfinder, self.clock)
        });
        let mut commands = self.handle_road_builder_result(result);
        commands.append(
            &mut self
                .avatar_artist
                .draw(&self.avatar, &self.world, &self.clock),
        );
        commands
    }

    fn build_house(&mut self) -> Vec<Command> {
        if let Some(mouse_coord) = self.mouse_coord {
            self.house_builder.build_house(
                &v2(
                    mouse_coord.x.floor() as usize,
                    mouse_coord.y.floor() as usize,
                ),
                &self.world,
            )
        } else {
            vec![]
        }
    }

    fn walk_to(&mut self) {
        if let Some(WorldCoord { x, y, .. }) = self.mouse_coord {
            let to = v2(x.round() as usize, y.round() as usize);
            self.avatar
                .walk_to(&self.world, &to, &self.avatar_pathfinder, self.clock);
        }
    }

    fn center(&self) -> Option<Command> {
        let x = self.world.width() / 2;
        let y = self.world.width() / 2;
        if let Some(cell) = self.world.get_cell(&v2(x, y)) {
            let z = cell.elevation();
            Some(Command::LookAt(WorldCoord::new(x as f32, y as f32, z)))
        } else {
            None
        }
    }

    fn add_label(&mut self) {
        if let Some(WorldCoord { x, y, .. }) = self.mouse_coord {
            let x = x.round() as usize;
            let y = y.round() as usize;
            if let Some(cell) = self.world.get_cell(&v2(x, y)) {
                self.label_editor
                    .start_edit(WorldCoord::new(x as f32, y as f32, cell.elevation()));
            }
        }
    }

    fn update_visiblity(&mut self) -> Vec<Command> {
        let visibility_computer = self.visibility_computer.update_visibility(
            &mut self.world,
            &self.clock,
            &self.avatar,
            310,
        );
        for position in visibility_computer.iter() {
            self.avatar_pathfinder.update_node(&self.world, position);
            self.road_builder
                .pathfinder()
                .update_node(&self.world, position);
        }
        if !visibility_computer.is_empty() {
            self.update_visited_layer();
        }
        self.world_artist
            .draw_affected(&self.world, &visibility_computer)
    }

    fn toggle_follow(&mut self) {
        self.follow_avatar = !self.follow_avatar;
        if self.follow_avatar {
            self.rotate_handler.rotate_over_undrawn();
        } else {
            self.rotate_handler.no_rotate_over_undrawn();
        }
    }

    fn create_visited_layer(&mut self) {
        let red = Some(Color::new(1.0, 0.0, 0.0, 1.0));
        let layer = Box::new(NodeTerrainColoring::new(M::from_fn(
            self.world.width(),
            self.world.height(),
            |x, y| {
                if point_has_been_visited(&self.world, &v2(x, y)) {
                    red
                } else {
                    None
                }
            },
        )));
        self.world_artist
            .coloring()
            .add_layer("visited".to_string(), layer, -2);
    }

    fn update_visited_layer(&mut self) {
        if self.world_artist.coloring().has_layer("visited") {
            self.create_visited_layer();
        }
    }

    fn toggle_visited_layer(&mut self) -> Vec<Command> {
        if self.world_artist.coloring().has_layer("visited") {
            self.world_artist.coloring().remove_layer("visited");
        } else {
            self.create_visited_layer();
        }
        self.world_artist.init(&self.world)
    }
}

impl EventHandler for GameHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        let mut commands = vec![];
        self.clock = Instant::now();
        self.avatar.evolve(&self.clock);

        let mut label_commands = self.label_editor.handle_event(event.clone());
        if !label_commands.is_empty() {
            commands.append(&mut label_commands);
        } else {
            match *event {
                Event::Start => {
                    commands.append(&mut self.label_editor.draw_all());
                    commands.append(&mut self.house_builder.rebuild_houses(&self.world));
                    commands.append(&mut self.world_artist.init(&self.world));
                    self.center()
                        .into_iter()
                        .for_each(|command| commands.push(command));
                }
                Event::WorldPositionChanged(mouse_coord) => {
                    if mouse_coord.x >= 0.0 && mouse_coord.y >= 0.0 && mouse_coord.z >= 0.0 {
                        self.mouse_coord = Some(mouse_coord);
                    } else {
                        self.mouse_coord = None;
                    }
                }
                Event::Key {
                    key,
                    state: ElementState::Pressed,
                    modifiers: ModifiersState { alt: false, .. },
                    ..
                } => match key {
                    VirtualKeyCode::W => {
                        self.avatar
                            .walk_forward(&self.world, &self.avatar_pathfinder, self.clock);
                    }
                    VirtualKeyCode::A => {
                        self.avatar.rotate_anticlockwise();
                    }
                    VirtualKeyCode::D => {
                        self.avatar.rotate_clockwise();
                    }
                    VirtualKeyCode::S => self.avatar.stop(&self.clock),
                    VirtualKeyCode::R => commands.append(&mut self.build_road()),
                    VirtualKeyCode::X => commands.append(&mut self.auto_build_road()),
                    VirtualKeyCode::L => self.add_label(),
                    VirtualKeyCode::H => commands.append(&mut self.build_house()),
                    VirtualKeyCode::C => self.toggle_follow(),
                    VirtualKeyCode::P => {
                        if let Some(save) = Save::new(&self) {
                            save.to_file("save");
                        }
                    }
                    VirtualKeyCode::O => commands.append(&mut self.toggle_overlay()),
                    VirtualKeyCode::V => commands.append(&mut self.toggle_visited_layer()),
                    _ => (),
                },
                Event::Key {
                    key,
                    state: ElementState::Pressed,
                    modifiers: ModifiersState { alt: true, .. },
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
                    VirtualKeyCode::V => {
                        self.world.reveal_all();
                        self.avatar_pathfinder.compute_network(&self.world);
                        self.road_builder.pathfinder().compute_network(&self.world);
                        commands.append(&mut self.world_artist.init(&self.world));
                    }
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
            commands.append(&mut self.rotate_handler.handle_event(event.clone()));
            if self.follow_avatar {
                if let Some(world_coord) = self.avatar.compute_world_coord(&self.world, &self.clock)
                {
                    commands.push(Command::LookAt(world_coord));
                }
            }
        }
        self.clock = Instant::now();
        commands.append(&mut self.update_visiblity());
        commands.append(
            &mut self
                .avatar_artist
                .draw(&self.avatar, &self.world, &self.clock),
        );
        commands
    }
}

#[derive(PartialEq, Debug, Serialize)]
struct Save<'a> {
    world: &'a World,
    world_gen_params: &'a WorldGenParameters,
    visibility_computer: &'a VisibilityComputer,
    avatar_position: &'a V2<usize>,
    avatar_rotation: &'a Rotation,
    labels: &'a HashMap<String, Label>,
    houses: &'a M<bool>,
}

impl<'a> Save<'a> {
    fn new(game_handler: &GameHandler) -> Option<Save> {
        if let Some(AvatarState::Stationary { position, rotation }) = game_handler.avatar.state() {
            Some(Save {
                world: &game_handler.world,
                world_gen_params: &game_handler.world_gen_params,
                visibility_computer: &game_handler.visibility_computer,
                avatar_position: position,
                avatar_rotation: rotation,
                labels: &game_handler.label_editor.labels(),
                houses: &game_handler.house_builder.houses(),
            })
        } else {
            None
        }
    }
    fn to_file(&self, file_name: &str) {
        let mut file = BufWriter::new(File::create(file_name).unwrap());
        bincode::serialize_into(&mut file, &self).unwrap();
    }
}

#[derive(PartialEq, Debug, Deserialize)]
pub struct Load {
    world: World,
    world_gen_params: WorldGenParameters,
    visibility_computer: VisibilityComputer,
    avatar_position: V2<usize>,
    avatar_rotation: Rotation,
    labels: HashMap<String, Label>,
    houses: M<bool>,
}

impl Load {
    pub fn from_file(file_name: &str) -> Load {
        let file = BufReader::new(File::open(file_name).unwrap());
        bincode::deserialize_from(file).unwrap()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::M;

    #[test]
    fn save_load_round_trip() {
        let world = World::new(
            M::from_vec(3, 3, vec![1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0]),
            0.5,
        );
        let game_handler =
            GameHandler::new(world, WorldGenParameters::default(), &mut thread_rng());
        let save = Save::new(&game_handler).unwrap();
        let encoded: Vec<u8> = bincode::serialize(&save).unwrap();
        let _: Load = bincode::deserialize(&encoded[..]).unwrap();
    }
}
