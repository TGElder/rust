use super::*;
use crate::world::World;
use commons::{na, v3, V3};
use isometric::coords::*;
use isometric::drawing::{
    create_billboard, create_boat, draw_boat, update_billboard, DrawBoatParams,
};
use isometric::Color;
use isometric::Command;
use std::collections::{HashMap, HashSet};
use std::iter::once;

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

pub struct AvatarArtist {
    params: AvatarArtistParams,
    body_parts: Vec<BodyPart>,
    last_draw_state: HashMap<String, AvatarDrawState>,
}

pub struct AvatarArtistParams {
    pixels_per_cell: f32,
    boat_params: DrawBoatParams,
}

impl AvatarArtistParams {
    fn new(light_direction: &V3<f32>) -> AvatarArtistParams {
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
        }
    }
}

struct BodyPart {
    offset: V3<f32>,
    handle: String,
    texture: String,
    texture_width: usize,
    texture_height: usize,
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

impl AvatarArtist {
    pub fn new(light_direction: &V3<f32>) -> AvatarArtist {
        AvatarArtist {
            params: AvatarArtistParams::new(light_direction),
            body_parts: vec![
                BodyPart {
                    offset: v3(0.0, 0.0, 96.0),
                    handle: "body".to_string(),
                    texture: "body.png".to_string(),
                    texture_width: 128,
                    texture_height: 198,
                },
                BodyPart {
                    offset: v3(12.0, 0.0, 192.0),
                    handle: "head".to_string(),
                    texture: "head.png".to_string(),
                    texture_width: 96,
                    texture_height: 96,
                },
                BodyPart {
                    offset: v3(48.0, 24.0, 192.0),
                    handle: "left_eye".to_string(),
                    texture: "eye.png".to_string(),
                    texture_width: 16,
                    texture_height: 16,
                },
                BodyPart {
                    offset: v3(48.0, -24.0, 192.0),
                    handle: "right_eye".to_string(),
                    texture: "eye.png".to_string(),
                    texture_width: 16,
                    texture_height: 16,
                },
                BodyPart {
                    offset: v3(48.0, 50.0, 96.0),
                    handle: "left_hand".to_string(),
                    texture: "hand.png".to_string(),
                    texture_width: 32,
                    texture_height: 32,
                },
                BodyPart {
                    offset: v3(48.0, -50.0, 96.0),
                    handle: "right_hand".to_string(),
                    texture: "hand.png".to_string(),
                    texture_width: 32,
                    texture_height: 32,
                },
            ],
            last_draw_state: HashMap::new(),
        }
    }

    pub fn init(&self, name: &str) -> Vec<Command> {
        self.body_parts
            .iter()
            .map(move |part| create_billboard(part_drawing_name(&name, &part), &part.texture))
            .chain(once(create_boat(boat_drawing_name(&name))))
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
            .flat_map(|Avatar { name, state, .. }| {
                self.draw_avatar(&name, state, world, instant, travel_mode_fn)
            })
            .collect()
    }

    fn draw_avatar(
        &mut self,
        name: &str,
        state: &AvatarState,
        world: &World,
        instant: &u128,
        travel_mode_fn: &TravelModeFn,
    ) -> Vec<Command> {
        let mut out = vec![];

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
            out.append(&mut self.draw_body(&name, state, instant, world_coord));
            out.append(&mut self.draw_boat_if_required(
                &name,
                state,
                world,
                world_coord,
                instant,
                travel_mode_fn,
            ));
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

    fn draw_body(
        &self,
        name: &str,
        state: &AvatarState,
        instant: &u128,
        world_coord: WorldCoord,
    ) -> Vec<Command> {
        self.body_parts
            .iter()
            .flat_map(|part| {
                self.draw_part_at_offset(&name, state, instant, world_coord, part)
                    .into_iter()
            })
            .collect()
    }

    fn draw_part_at_offset(
        &self,
        name: &str,
        state: &AvatarState,
        instant: &u128,
        world_coord: WorldCoord,
        part: &BodyPart,
    ) -> Vec<Command> {
        let offset = AvatarArtist::get_rotation_matrix(state, instant) * part.offset
            / self.params.pixels_per_cell;
        let world_coord = WorldCoord::new(
            world_coord.x + offset.x,
            world_coord.y + offset.y,
            world_coord.z + offset.z,
        );
        let width = (part.texture_width as f32) / self.params.pixels_per_cell;
        let height = (part.texture_height as f32) / self.params.pixels_per_cell;
        update_billboard(part_drawing_name(&name, &part), world_coord, width, height)
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

    fn hide(&self, name: &str) -> Vec<Command> {
        self.body_parts
            .iter()
            .map(|part| self.hide_part(name, part))
            .chain(once(self.hide_boat(name)))
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

    fn erase(&self, name: &str) -> Vec<Command> {
        self.body_parts
            .iter()
            .map(|part| self.erase_part(name, part))
            .chain(once(self.erase_boat(name)))
            .collect()
    }

    fn erase_part(&self, name: &str, part: &BodyPart) -> Command {
        Command::Erase(part_drawing_name(name, part))
    }

    fn erase_boat(&self, name: &str) -> Command {
        Command::Erase(boat_drawing_name(name))
    }
}
