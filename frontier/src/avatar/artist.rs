use super::*;
use crate::world::World;
use commons::{na, v3, V3};
use isometric::coords::*;
use isometric::drawing::{draw_billboard, draw_boat, DrawBoatParams};
use isometric::Color;
use isometric::Command;

pub struct AvatarArtist {
    scale: f32,
    boat_params: DrawBoatParams,
    body_parts: Vec<BodyPart>,
}

struct BodyPart {
    offset: V3<f32>,
    handle: String,
    texture: String,
    texture_width: usize,
    texture_height: usize,
}

impl AvatarArtist {
    pub fn new(scale: f32, light_direction: V3<f32>) -> AvatarArtist {
        AvatarArtist {
            scale,
            boat_params: DrawBoatParams {
                width: 0.12,
                side_height: 0.04,
                bow_length: 0.06,
                mast_height: 0.4,
                base_color: Color::new(0.46875, 0.257_812_5, 0.070_312_5, 0.8),
                sail_color: Color::new(1.0, 1.0, 1.0, 1.0),
                light_direction,
            },
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
    fn get_rotation_matrix(avatar: &Avatar, instant: &Instant) -> na::Matrix3<f32> {
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
        avatar: &Avatar,
        instant: &Instant,
        world_coord: WorldCoord,
        part: &BodyPart,
    ) -> Vec<Command> {
        let offset = AvatarArtist::get_rotation_matrix(avatar, instant) * part.offset * self.scale;
        let world_coord = WorldCoord::new(
            world_coord.x + offset.x,
            world_coord.y + offset.y,
            world_coord.z + offset.z,
        );
        let width = (part.texture_width as f32) * self.scale;
        let height = (part.texture_height as f32) * self.scale;
        draw_billboard(
            part.handle.clone(),
            world_coord,
            width,
            height,
            &part.texture,
        )
    }

    pub fn draw(&self, avatar: &Avatar, world: &World, instant: &Instant) -> Vec<Command> {
        let mut out = self.draw_boat_if_required(avatar, world, instant);
        if let Some(world_coord) = avatar.compute_world_coord(world, instant) {
            for part in self.body_parts.iter() {
                out.append(&mut self.draw_billboard_at_offset(avatar, instant, world_coord, part));
            }
        }
        out
    }

    fn draw_boat_if_required(
        &self,
        avatar: &Avatar,
        world: &World,
        instant: &Instant,
    ) -> Vec<Command> {
        if let Some(world_coord) = avatar.compute_world_coord(world, instant) {
            let check_position = v2(
                world_coord.x.round() as usize,
                world_coord.y.round() as usize,
            );
            match avatar
                .travel_mode_fn
                .travel_mode_here(world, &check_position)
            {
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
        avatar: &Avatar,
        world_coord: WorldCoord,
        instant: &Instant,
    ) -> Vec<Command> {
        draw_boat(
            "boat",
            world_coord,
            AvatarArtist::get_rotation_matrix(avatar, instant),
            &self.boat_params,
        )
    }
}
