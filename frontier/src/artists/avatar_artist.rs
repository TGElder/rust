use super::*;
use crate::avatar::*;
use commons::log::debug;
use isometric::coords::*;
use isometric::drawing::{
    create_billboard, update_billboard_texture, update_billboard_vertices, Billboard,
};
use isometric::Command;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::iter::once;

const SPRITE_SHEET_PNG: &str = "resources/textures/sprite_sheets/resources.png";
const SPRITE_SHEET_JSON: &str = "resources/textures/sprite_sheets/resources.json";

pub struct AvatarArtist {
    params: AvatarArtistParams,
    previous_draw_actions: HashMap<String, AvatarDrawAction>,
}

pub struct AvatarArtistParams {
    load_size: f32,
    load_height: f32,
    texture_coords: HashMap<String, (V2<f32>, V2<f32>)>,
}

impl AvatarArtistParams {
    pub fn new() -> AvatarArtistParams {
        let file = File::open(SPRITE_SHEET_JSON).unwrap();
        let reader = BufReader::new(file);
        let sprite_sheet: SpriteSheet = serde_json::from_reader(reader).unwrap();
        let texture_coords = sprite_sheet.into();
        debug!("{:?}", texture_coords);
        AvatarArtistParams {
            load_size: 0.15,
            load_height: 0.3,
            texture_coords,
        }
    }
}

pub struct AvatarDrawCommand<'a> {
    pub avatar: &'a Avatar,
    pub draw_when_done: bool,
}

impl AvatarArtist {
    pub fn new(params: AvatarArtistParams) -> AvatarArtist {
        AvatarArtist {
            params,
            previous_draw_actions: HashMap::new(),
        }
    }

    pub fn init(&self, name: &str) -> Vec<Command> {
        let drawing_name = load_drawing_name(&name);
        vec![
            create_billboard(drawing_name.clone()),
            update_billboard_texture(drawing_name, SPRITE_SHEET_PNG),
        ]
    }

    pub fn update_avatars(
        &mut self,
        commands: &[AvatarDrawCommand],
        instant: &u128,
    ) -> Vec<Command> {
        let mut out = vec![];
        out.append(&mut self.draw_avatars(commands, instant));
        out
    }

    fn draw_avatars(&mut self, commands: &[AvatarDrawCommand], instant: &u128) -> Vec<Command> {
        commands
            .iter()
            .flat_map(|command| self.draw_command(command, instant))
            .collect()
    }

    fn draw_command(&mut self, command: &AvatarDrawCommand, instant: &u128) -> Vec<Command> {
        let mut out = vec![];
        let avatar = command.avatar;
        let name = &avatar.name;
        let new_draw_action = avatar_draw_action(&command, &instant);
        let previous_draw_action = self.previous_draw_actions.get(name);
        if let Some(previous_draw_action) = previous_draw_action {
            if !Self::should_redraw_avatar(&previous_draw_action, &new_draw_action) {
                return vec![];
            }
        } else {
            out.append(&mut self.init(name));
        }
        self.previous_draw_actions
            .insert(name.to_string(), new_draw_action);

        match new_draw_action {
            AvatarDrawAction::Draw => out.append(&mut self.draw_avatar(avatar, instant)),
            AvatarDrawAction::Hide => out.append(&mut self.hide(name)),
        }
        out
    }

    fn draw_avatar(&self, avatar: &Avatar, instant: &u128) -> Vec<Command> {
        let journey = avatar.journey.as_ref().unwrap();
        let world_coord = journey.world_coord_at(instant);
        let mut out = vec![];
        out.append(&mut self.draw_load(
            &avatar.name,
            &journey.progress_at(instant).load(),
            world_coord,
        ));
        out
    }

    fn should_redraw_avatar(
        previous_draw_action: &AvatarDrawAction,
        new_draw_action: &AvatarDrawAction,
    ) -> bool {
        if let AvatarDrawAction::Draw = new_draw_action {
            true
        } else {
            previous_draw_action != new_draw_action
        }
    }

    fn draw_load(
        &self,
        name: &str,
        load: &AvatarLoad,
        mut world_coord: WorldCoord,
    ) -> Vec<Command> {
        if let AvatarLoad::Resource(resource) = load {
            let (texture_from, texture_to) = unwrap_or!(
                self.params.texture_coords.get(resource.name()),
                return vec![self.hide_load(name)]
            );
            let mut out = vec![];
            let name = load_drawing_name(name);
            world_coord.z += self.params.load_height;
            out.append(&mut update_billboard_vertices(
                name,
                Billboard {
                    world_coord: &world_coord,
                    width: &self.params.load_size,
                    height: &self.params.load_size,
                    texture_from: &texture_from,
                    texture_to: &texture_to,
                },
            ));
            out
        } else {
            vec![self.hide_load(name)]
        }
    }

    fn hide(&self, name: &str) -> Vec<Command> {
        once(self.hide_load(name)).collect()
    }

    fn hide_load(&self, name: &str) -> Command {
        Command::SetDrawingVisibility {
            name: load_drawing_name(name),
            visible: false,
        }
    }
}

fn drawing_name(name: &str, part: &str) -> String {
    format!("avatar-{}-{}", name.to_string(), part)
}

fn load_drawing_name(name: &str) -> String {
    drawing_name(name, "load")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AvatarDrawAction {
    Draw,
    Hide,
}

fn avatar_draw_action(command: &AvatarDrawCommand, instant: &u128) -> AvatarDrawAction {
    match &command.avatar.journey {
        Some(journey) => match command.draw_when_done || !journey.done(instant) {
            true => AvatarDrawAction::Draw,
            false => AvatarDrawAction::Hide,
        },
        None => AvatarDrawAction::Hide,
    }
}

#[derive(Debug, Deserialize)]
struct SpriteSheet {
    frames: HashMap<String, Sprite>,
    meta: Meta,
}

impl Into<HashMap<String, (V2<f32>, V2<f32>)>> for SpriteSheet {
    fn into(self) -> HashMap<String, (V2<f32>, V2<f32>)> {
        let w = self.meta.size.w as f32;
        let h = self.meta.size.h as f32;

        self.frames
            .into_iter()
            .map(|(name, sprite)| {
                (
                    name,
                    (
                        v2(sprite.frame.x as f32 / w, sprite.frame.y as f32 / h),
                        v2(
                            (sprite.frame.x + sprite.frame.w) as f32 / w,
                            (sprite.frame.y + sprite.frame.h) as f32 / h,
                        ),
                    ),
                )
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct Sprite {
    frame: Frame,
}

#[derive(Debug, Deserialize)]

struct Frame {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

#[derive(Debug, Deserialize)]

struct Meta {
    size: Size,
}

#[derive(Debug, Deserialize)]

struct Size {
    w: usize,
    h: usize,
}
