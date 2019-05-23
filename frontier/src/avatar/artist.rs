use super::*;
use crate::world::World;
use commons::{v3, V3};
use isometric::coords::*;
use isometric::drawing::draw_billboard;
use isometric::Command;

pub struct AvatarArtist {
    scale: f32,
}

impl AvatarArtist {
    pub fn new(scale: f32) -> AvatarArtist {
        AvatarArtist { scale }
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
        texture: &str,
        texture_width: usize,
        texture_height: usize,
    ) -> Vec<Command> {
        let offset = AvatarArtist::get_rotation_matrix(avatar, world) * offset * self.scale;
        let world_coord = WorldCoord::new(
            world_coord.x + offset.x,
            world_coord.y + offset.y,
            world_coord.z + offset.z,
        );
        let width = (texture_width as f32) * self.scale;
        let height = (texture_height as f32) * self.scale;
        draw_billboard(handle.to_string(), world_coord, width, height, texture)
    }

    pub fn draw(&self, avatar: &Avatar, world: &World) -> Vec<Command> {
        if let Some(world_coord) = avatar.compute_world_coord(world) {
            let mut out = vec![];

            out.append(&mut self.draw_billboard_at_offset(
                avatar,
                world,
                world_coord,
                v3(0.0, 0.0, 96.0),
                "body",
                "body.png",
                128,
                198,
            ));
            out.append(&mut self.draw_billboard_at_offset(
                avatar,
                world,
                world_coord,
                v3(12.0, 0.0, 192.0),
                "head",
                "head.png",
                96,
                96,
            ));
            out.append(&mut self.draw_billboard_at_offset(
                avatar,
                world,
                world_coord,
                v3(48.0, 24.0, 192.0),
                "left_eye",
                "eye.png",
                16,
                16,
            ));
            out.append(&mut self.draw_billboard_at_offset(
                avatar,
                world,
                world_coord,
                v3(48.0, -24.0, 192.0),
                "right_eye",
                "eye.png",
                16,
                16,
            ));
            out.append(&mut self.draw_billboard_at_offset(
                avatar,
                world,
                world_coord,
                v3(48.0, 50.0, 96.0),
                "left_hand",
                "hand.png",
                32,
                32,
            ));
            out.append(&mut self.draw_billboard_at_offset(
                avatar,
                world,
                world_coord,
                v3(48.0, -50.0, 96.0),
                "right_hand",
                "hand.png",
                32,
                32,
            ));
            out
        } else {
            vec![]
        }
    }
}
