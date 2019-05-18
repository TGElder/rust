use super::*;
use crate::world::World;
use commons::{v3, V3};
use isometric::coords::*;
use isometric::drawing::Billboard;
use isometric::Command;
use isometric::Texture;
use std::sync::Arc;

pub struct AvatarArtist {
    scale: f32,
    texture_body: Arc<Texture>,
    texture_head: Arc<Texture>,
    texture_eye: Arc<Texture>,
    texture_hand: Arc<Texture>,
}

impl AvatarArtist {
    pub fn new(scale: f32) -> AvatarArtist {
        AvatarArtist {
            scale,
            texture_body: Arc::new(Texture::new(image::open("body.png").unwrap())),
            texture_head: Arc::new(Texture::new(image::open("head.png").unwrap())),
            texture_eye: Arc::new(Texture::new(image::open("eye.png").unwrap())),
            texture_hand: Arc::new(Texture::new(image::open("hand.png").unwrap())),
        }
    }

    #[rustfmt::skip]
    fn get_rotation_matrix(avatar: &Avatar, world: &World) -> na::Matrix3<f32> {
        let rotation = avatar.rotation(world).unwrap_or(Rotation::Up);
        let cos = rotation.angle().cos();
        let sin = rotation.angle().sin();
        na::Matrix3::from_vec(vec![
            cos, sin, 0.0,
            -sin, cos, 0.0,
            0.0, 0.0, 1.0,
        ])
    }

    pub fn draw_billboard_at_offset(
        &self,
        avatar: &Avatar,
        world: &World,
        world_coord: WorldCoord,
        offset: V3<f32>,
        handle: &str,
        texture: &Arc<Texture>,
    ) -> Command {
        let offset = AvatarArtist::get_rotation_matrix(avatar, world) * offset * self.scale;
        let world_coord = WorldCoord::new(
            world_coord.x + offset.x,
            world_coord.y + offset.y,
            world_coord.z + offset.z,
        );
        let width = (texture.width() as f32) * self.scale;
        let height = (texture.height() as f32) * self.scale;
        Command::Draw {
            name: handle.to_string(),
            drawing: Box::new(Billboard::new(world_coord, width, height, texture.clone())),
        }
    }

    pub fn draw(&self, avatar: &Avatar, world: &World) -> Vec<Command> {
        if let Some(world_coord) = avatar.compute_world_coord(world) {
            vec![
                self.draw_billboard_at_offset(
                    avatar,
                    world,
                    world_coord,
                    v3(0.0, 0.0, 96.0),
                    "body",
                    &self.texture_body,
                ),
                self.draw_billboard_at_offset(
                    avatar,
                    world,
                    world_coord,
                    v3(12.0, 0.0, 192.0),
                    "head",
                    &self.texture_head,
                ),
                self.draw_billboard_at_offset(
                    avatar,
                    world,
                    world_coord,
                    v3(48.0, 24.0, 192.0),
                    "left_eye",
                    &self.texture_eye,
                ),
                self.draw_billboard_at_offset(
                    avatar,
                    world,
                    world_coord,
                    v3(48.0, -24.0, 192.0),
                    "right_eye",
                    &self.texture_eye,
                ),
                self.draw_billboard_at_offset(
                    avatar,
                    world,
                    world_coord,
                    v3(48.0, 50.0, 96.0),
                    "left_hand",
                    &self.texture_hand,
                ),
                self.draw_billboard_at_offset(
                    avatar,
                    world,
                    world_coord,
                    v3(48.0, -50.0, 96.0),
                    "right_hand",
                    &self.texture_hand,
                ),
            ]
        } else {
            vec![]
        }
    }
}
