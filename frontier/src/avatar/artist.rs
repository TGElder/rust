use super::*;
use crate::world::World;
use commons::{na, v3, V3};
use isometric::coords::*;
use isometric::drawing::{draw_billboard, draw_boat, DrawBoatParams};
use isometric::Color;
use isometric::Command;

pub struct AvatarArtist {
    params: AvatarArtistParams,
    body_parts: Vec<BodyPart>,
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
        }
    }

    #[rustfmt::skip]
    fn get_rotation_matrix(avatar: &AvatarState, instant: &u128) -> na::Matrix3<f32> {
        let rotation = avatar.rotation(instant).unwrap_or(Rotation::Up);
        let cos = rotation.angle().cos();
        let sin = rotation.angle().sin();
        na::Matrix3::from_vec(vec![
            cos, sin, 0.0,
            -sin, cos, 0.0,
            0.0, 0.0, 1.0,
        ])
    }

    fn draw_billboard_at_offset(
        &self,
        avatar: &AvatarState,
        instant: &u128,
        world_coord: WorldCoord,
        part: &BodyPart,
    ) -> Vec<Command> {
        let offset = AvatarArtist::get_rotation_matrix(avatar, instant) * part.offset
            / self.params.pixels_per_cell;
        let world_coord = WorldCoord::new(
            world_coord.x + offset.x,
            world_coord.y + offset.y,
            world_coord.z + offset.z,
        );
        let width = (part.texture_width as f32) / self.params.pixels_per_cell;
        let height = (part.texture_height as f32) / self.params.pixels_per_cell;
        draw_billboard(
            part.handle.clone(),
            world_coord,
            width,
            height,
            &part.texture,
        )
    }

    pub fn draw(
        &self,
        avatar: &AvatarState,
        world: &World,
        instant: &u128,
        travel_mode_fn: &TravelModeFn,
    ) -> Vec<Command> {
        let mut out = self.draw_boat_if_required(avatar, world, instant, travel_mode_fn);
        if let Some(world_coord) = avatar.compute_world_coord(world, instant) {
            for part in self.body_parts.iter() {
                out.append(&mut self.draw_billboard_at_offset(avatar, instant, world_coord, part));
            }
        }
        out
    }

    fn draw_boat_if_required(
        &self,
        avatar: &AvatarState,
        world: &World,
        instant: &u128,
        travel_mode_fn: &TravelModeFn,
    ) -> Vec<Command> {
        if let Some(world_coord) = avatar.compute_world_coord(world, instant) {
            let travel_mode = match avatar {
                AvatarState::Walking { .. } => {
                    let from = v2(
                        world_coord.x.floor() as usize,
                        world_coord.y.floor() as usize,
                    );
                    let to = v2(world_coord.x.ceil() as usize, world_coord.y.ceil() as usize);
                    travel_mode_fn.travel_mode_between(world, &from, &to)
                }
                AvatarState::Stationary { position, .. } => {
                    travel_mode_fn.travel_mode_here(world, &position)
                }
                _ => None,
            };
            match travel_mode {
                Some(TravelMode::Sea) => self.draw_boat(avatar, world_coord, instant),
                Some(TravelMode::River) => self.draw_boat(avatar, world_coord, instant),
                _ => vec![],
            }
        } else {
            vec![]
        }
    }

    fn draw_boat(
        &self,
        avatar: &AvatarState,
        world_coord: WorldCoord,
        instant: &u128,
    ) -> Vec<Command> {
        draw_boat(
            "boat",
            world_coord,
            AvatarArtist::get_rotation_matrix(avatar, instant),
            &self.params.boat_params,
        )
    }
}
