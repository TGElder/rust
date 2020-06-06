use super::*;
use crate::avatar::*;
use crate::world::World;
use commons::{na, v3, V3};
use isometric::coords::*;
use isometric::drawing::{
    create_billboard, create_boat, create_masked_billboard, draw_boat, update_billboard_texture,
    update_billboard_vertices, update_masked_billboard_mask, update_masked_billboard_texture,
    update_masked_billboard_vertices, DrawBoatParams,
};
use isometric::Color;
use isometric::Command;
use std::collections::{HashMap, HashSet};
use std::iter::once;

pub struct AvatarArtistParams {
    pixels_per_cell: f32,
    boat_params: DrawBoatParams,
    load_size: f32,
    load_height: f32,
}

impl AvatarArtistParams {
    pub fn new(light_direction: &V3<f32>) -> AvatarArtistParams {
        AvatarArtistParams {
            pixels_per_cell: 1280.0,
            boat_params: DrawBoatParams {
                width: 0.12,
                side_height: 0.04,
                bow_length: 0.06,
                mast_height: 0.4,
                base_color: Color::new(0.46875, 0.257_812_5, 0.070_312_5, 0.8),
                sail_color: Color::new(1.0, 1.0, 1.0, 1.0),
                light_direction: *light_direction,
            },
            load_size: 0.15,
            load_height: 0.3,
        }
    }
}

pub struct AvatarArtist {
    params: AvatarArtistParams,
    body_parts: Vec<BodyPart>,
    last_draw_state: HashMap<String, AvatarDrawState>,
}

impl AvatarArtist {
    pub fn new(params: AvatarArtistParams) -> AvatarArtist {
        AvatarArtist {
            params,
            body_parts: vec![
                BodyPart {
                    offset: v3(0.0, 0.0, 96.0),
                    handle: "body".to_string(),
                    texture: "resources/textures/body.png".to_string(),
                    texture_width: 128,
                    texture_height: 198,
                    mask: Some(ColorMask {
                        mask: "resources/textures/body.png".to_string(),
                        color: AvatarColor::Base,
                    }),
                },
                BodyPart {
                    offset: v3(12.0, 0.0, 192.0),
                    handle: "head".to_string(),
                    texture: "resources/textures/head.png".to_string(),
                    texture_width: 96,
                    texture_height: 96,
                    mask: Some(ColorMask {
                        mask: "resources/textures/head.png".to_string(),
                        color: AvatarColor::Skin,
                    }),
                },
                BodyPart {
                    offset: v3(48.0, 24.0, 192.0),
                    handle: "left_eye".to_string(),
                    texture: "resources/textures/eye.png".to_string(),
                    texture_width: 16,
                    texture_height: 16,
                    mask: None,
                },
                BodyPart {
                    offset: v3(48.0, -24.0, 192.0),
                    handle: "right_eye".to_string(),
                    texture: "resources/textures/eye.png".to_string(),
                    texture_width: 16,
                    texture_height: 16,
                    mask: None,
                },
                BodyPart {
                    offset: v3(48.0, 50.0, 96.0),
                    handle: "left_hand".to_string(),
                    texture: "resources/textures/hand.png".to_string(),
                    texture_width: 32,
                    texture_height: 32,
                    mask: Some(ColorMask {
                        mask: "resources/textures/hand.png".to_string(),
                        color: AvatarColor::Skin,
                    }),
                },
                BodyPart {
                    offset: v3(48.0, -50.0, 96.0),
                    handle: "right_hand".to_string(),
                    texture: "resources/textures/hand.png".to_string(),
                    texture_width: 32,
                    texture_height: 32,
                    mask: Some(ColorMask {
                        mask: "resources/textures/hand.png".to_string(),
                        color: AvatarColor::Skin,
                    }),
                },
            ],
            last_draw_state: HashMap::new(),
        }
    }

    pub fn init(&self, name: &str) -> Vec<Command> {
        self.body_parts
            .iter()
            .flat_map(move |part| create_part_drawing(name, part))
            .chain(once(create_boat(boat_drawing_name(&name))))
            .chain(once(create_billboard(load_drawing_name(&name))))
            .collect()
    }

    pub fn update_avatars(
        &mut self,
        avatars: &HashMap<String, Avatar>,
        world: &World,
        instant: &u128,
        travel_mode_fn: &TravelModeFn,
    ) -> Vec<Command> {
        let mut out = vec![];
        out.append(&mut self.draw_avatars(avatars, world, instant, travel_mode_fn));
        out.append(&mut self.erase_avatars(avatars));
        out
    }

    fn draw_avatars(
        &mut self,
        avatars: &HashMap<String, Avatar>,
        world: &World,
        instant: &u128,
        travel_mode_fn: &TravelModeFn,
    ) -> Vec<Command> {
        avatars
            .values()
            .flat_map(|avatar| self.draw_avatar(avatar, world, instant, travel_mode_fn))
            .collect()
    }

    fn draw_avatar(
        &mut self,
        avatar: &Avatar,
        world: &World,
        instant: &u128,
        travel_mode_fn: &TravelModeFn,
    ) -> Vec<Command> {
        let mut out = vec![];
        let name = &avatar.name;
        let state = &avatar.state;
        let new_draw_state = avatar_draw_state(&state);
        let last_draw_state = self.last_draw_state.get(name);
        if let Some(last_draw_state) = last_draw_state {
            if !Self::should_redraw_avatar(&last_draw_state, &new_draw_state) {
                return vec![];
            }
        } else {
            out.append(&mut self.init(name));
        }
        self.last_draw_state
            .insert(name.to_string(), new_draw_state);

        if let Some(world_coord) = state.compute_world_coord(world, instant) {
            out.append(&mut self.draw_body(&avatar, instant, world_coord));
            out.append(&mut self.draw_boat_if_required(
                &name,
                state,
                world,
                world_coord,
                instant,
                travel_mode_fn,
            ));
            out.append(&mut self.draw_load(&name, &avatar.load, world_coord));
        } else {
            out.append(&mut self.hide(name));
        }
        out
    }

    fn erase_avatars(&mut self, avatars: &HashMap<String, Avatar>) -> Vec<Command> {
        let mut to_erase: HashSet<String> = self.last_draw_state.keys().cloned().collect();
        to_erase.retain(|avatar| !avatars.contains_key(avatar));
        self.last_draw_state
            .retain(|avatar, _| !to_erase.contains(avatar));
        to_erase
            .drain()
            .flat_map(|avatar| self.erase(&avatar))
            .collect()
    }

    #[rustfmt::skip]
    fn get_rotation_matrix(state: &AvatarState, instant: &u128) -> na::Matrix3<f32> {
        let rotation = state.rotation(instant).unwrap_or_default();
        let cos = rotation.angle().cos();
        let sin = rotation.angle().sin();
        na::Matrix3::from_vec(vec![
            cos, sin, 0.0,
            -sin, cos, 0.0,
            0.0, 0.0, 1.0,
        ])
    }

    fn should_redraw_avatar(
        last_draw_state: &AvatarDrawState,
        new_draw_state: &AvatarDrawState,
    ) -> bool {
        if let AvatarDrawState::Moving = new_draw_state {
            true
        } else {
            last_draw_state != new_draw_state
        }
    }

    fn draw_body(&self, avatar: &Avatar, instant: &u128, world_coord: WorldCoord) -> Vec<Command> {
        self.body_parts
            .iter()
            .flat_map(|part| {
                self.draw_part_at_offset(avatar, instant, world_coord, part)
                    .into_iter()
            })
            .collect()
    }

    fn draw_part_at_offset(
        &self,
        avatar: &Avatar,
        instant: &u128,
        world_coord: WorldCoord,
        part: &BodyPart,
    ) -> Vec<Command> {
        let offset = AvatarArtist::get_rotation_matrix(&avatar.state, instant) * part.offset
            / self.params.pixels_per_cell;
        let world_coord = WorldCoord::new(
            world_coord.x + offset.x,
            world_coord.y + offset.y,
            world_coord.z + offset.z,
        );
        let width = (part.texture_width as f32) / self.params.pixels_per_cell;
        let height = (part.texture_height as f32) / self.params.pixels_per_cell;
        if let Some(mask) = &part.mask {
            let color = mask.color.get(avatar);
            update_masked_billboard_vertices(
                part_drawing_name(&avatar.name, &part),
                world_coord,
                width,
                height,
                color,
            )
        } else {
            update_billboard_vertices(
                part_drawing_name(&avatar.name, &part),
                world_coord,
                width,
                height,
            )
        }
    }

    fn draw_boat_if_required(
        &self,
        name: &str,
        state: &AvatarState,
        world: &World,
        world_coord: WorldCoord,
        instant: &u128,
        travel_mode_fn: &TravelModeFn,
    ) -> Vec<Command> {
        if self.should_draw_boat(state, world, world_coord, travel_mode_fn) {
            self.draw_boat(name, state, world_coord, instant)
        } else {
            vec![self.hide_boat(name)]
        }
    }

    fn should_draw_boat(
        &self,
        state: &AvatarState,
        world: &World,
        world_coord: WorldCoord,
        travel_mode_fn: &TravelModeFn,
    ) -> bool {
        let travel_modes = match state {
            AvatarState::Walking { .. } => {
                let from = world_coord.to_v2_floor();
                let to = world_coord.to_v2_ceil();
                travel_mode_fn
                    .travel_mode_between(world, &from, &to)
                    .map(|mode| vec![mode])
                    .unwrap_or_default()
            }
            AvatarState::Stationary { position, .. } => {
                travel_mode_fn.travel_modes_here(world, &position)
            }
            _ => vec![],
        };
        !travel_modes
            .iter()
            .map(|mode| mode.class())
            .any(|class| class == TravelModeClass::Land)
    }

    fn draw_boat(
        &self,
        name: &str,
        state: &AvatarState,
        world_coord: WorldCoord,
        instant: &u128,
    ) -> Vec<Command> {
        draw_boat(
            &boat_drawing_name(name),
            world_coord,
            AvatarArtist::get_rotation_matrix(state, instant),
            &self.params.boat_params,
        )
    }

    fn draw_load(
        &self,
        name: &str,
        load: &AvatarLoad,
        mut world_coord: WorldCoord,
    ) -> Vec<Command> {
        if let AvatarLoad::Resource(resource) = load {
            let texture = unwrap_or!(
                resource_texture(*resource),
                return vec![self.hide_load(name)]
            );
            let mut out = vec![];
            let name = load_drawing_name(name);
            world_coord.z += self.params.load_height;
            out.append(&mut update_billboard_vertices(
                name.clone(),
                world_coord,
                self.params.load_size,
                self.params.load_size,
            ));
            out.append(&mut update_billboard_texture(name, texture));
            out
        } else {
            vec![self.hide_load(name)]
        }
    }

    fn hide(&self, name: &str) -> Vec<Command> {
        self.body_parts
            .iter()
            .map(|part| self.hide_part(name, part))
            .chain(once(self.hide_boat(name)))
            .chain(once(self.hide_load(name)))
            .collect()
    }

    fn hide_part(&self, name: &str, part: &BodyPart) -> Command {
        Command::SetDrawingVisibility {
            name: part_drawing_name(name, part),
            visible: false,
        }
    }

    fn hide_boat(&self, name: &str) -> Command {
        Command::SetDrawingVisibility {
            name: boat_drawing_name(name),
            visible: false,
        }
    }

    fn hide_load(&self, name: &str) -> Command {
        Command::SetDrawingVisibility {
            name: load_drawing_name(name),
            visible: false,
        }
    }

    fn erase(&self, name: &str) -> Vec<Command> {
        self.body_parts
            .iter()
            .map(|part| self.erase_part(name, part))
            .chain(once(self.erase_boat(name)))
            .chain(once(self.erase_load(name)))
            .collect()
    }

    fn erase_part(&self, name: &str, part: &BodyPart) -> Command {
        Command::Erase(part_drawing_name(name, part))
    }

    fn erase_boat(&self, name: &str) -> Command {
        Command::Erase(boat_drawing_name(name))
    }

    fn erase_load(&self, name: &str) -> Command {
        Command::Erase(load_drawing_name(name))
    }
}

fn drawing_name(name: &str, part: &str) -> String {
    format!("avatar-{}-{}", name.to_string(), part)
}

fn part_drawing_name(name: &str, part: &BodyPart) -> String {
    drawing_name(name, &part.handle)
}

fn boat_drawing_name(name: &str) -> String {
    drawing_name(name, "boat")
}

fn load_drawing_name(name: &str) -> String {
    drawing_name(name, "load")
}

fn create_part_drawing<'a>(
    name: &'a str,
    part: &'a BodyPart,
) -> Box<dyn Iterator<Item = Command> + 'a> {
    let name = part_drawing_name(&name, &part);
    if let Some(ColorMask { mask, .. }) = &part.mask {
        Box::new(create_masked_billboard_part_drawing(
            name,
            &part.texture,
            mask,
        ))
    } else {
        Box::new(create_billboard_part_drawing(name, &part.texture))
    }
}

fn create_masked_billboard_part_drawing<'a>(
    name: String,
    texture: &'a str,
    mask: &'a str,
) -> impl Iterator<Item = Command> + 'a {
    once(create_masked_billboard(name.clone()))
        .chain(update_masked_billboard_texture(name.clone(), &texture))
        .chain(update_masked_billboard_mask(name, &mask))
}

fn create_billboard_part_drawing<'a>(
    name: String,
    texture: &'a str,
) -> impl Iterator<Item = Command> + 'a {
    once(create_billboard(name.clone())).chain(update_billboard_texture(name, &texture))
}

fn resource_texture(resource: Resource) -> Option<&'static str> {
    match resource {
        Resource::Bananas => Some("resources/textures/bananas.png"),
        Resource::Coal => Some("resources/textures/coal.png"),
        Resource::Crabs => Some("resources/textures/crabs.png"),
        Resource::Deer => Some("resources/textures/deer.png"),
        Resource::Farmland => Some("resources/textures/wheat.png"),
        Resource::Fur => Some("resources/textures/fur.png"),
        Resource::Gems => Some("resources/textures/gems.png"),
        Resource::Gold => Some("resources/textures/gold.png"),
        Resource::Iron => Some("resources/textures/iron.png"),
        Resource::Ivory => Some("resources/textures/ivory.png"),
        Resource::Spice => Some("resources/textures/spice.png"),
        Resource::Stone => Some("resources/textures/stone.png"),
        Resource::Truffles => Some("resources/textures/truffles.png"),
        Resource::Whales => Some("resources/textures/whales.png"),
        Resource::Wood => Some("resources/textures/wood.png"),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AvatarDrawState {
    Stationary {
        position: V2<usize>,
        rotation: Rotation,
    },
    Moving,
    Absent,
}

fn avatar_draw_state(state: &AvatarState) -> AvatarDrawState {
    match state {
        AvatarState::Stationary {
            position, rotation, ..
        } => AvatarDrawState::Stationary {
            position: *position,
            rotation: *rotation,
        },
        AvatarState::Absent => AvatarDrawState::Absent,
        AvatarState::Walking(..) => AvatarDrawState::Moving,
    }
}

struct BodyPart {
    offset: V3<f32>,
    handle: String,
    texture: String,
    texture_width: usize,
    texture_height: usize,
    mask: Option<ColorMask>,
}

struct ColorMask {
    color: AvatarColor,
    mask: String,
}

enum AvatarColor {
    Base,
    Skin,
}

impl AvatarColor {
    fn get<'a>(&'a self, avatar: &'a Avatar) -> &'a Color {
        match self {
            AvatarColor::Base => &avatar.color,
            AvatarColor::Skin => &avatar.skin_color,
        }
    }
}
