use std::iter::once;

use commons::na::Matrix3;
use commons::V3;
use isometric::coords::WorldCoord;
use isometric::drawing::{
    create_billboards, create_masked_billboards, update_billboard_texture,
    update_billboards_vertices, update_masked_billboard_mask, update_masked_billboard_texture,
    update_masked_billboards_vertices,
};
use isometric::{Color, Command};

use crate::avatar::Avatar;

use super::artist_avatar::ArtistAvatar;

pub struct BodyPartArtist {
    body_part: BodyPart,
    world_width: f32,
    world_height: f32,
    offsets: [V3<f32>; 4],
}

#[derive(Clone)]
pub struct BodyPart {
    pub offset: V3<f32>,
    pub drawing_name: &'static str,
    pub texture: &'static str,
    pub texture_width: usize,
    pub texture_height: usize,
    pub mask: Option<ColorMask>,
}

#[derive(Clone)]
pub struct ColorMask {
    pub mask: &'static str,
    pub color_fn: fn(&Avatar) -> &Color,
}

impl BodyPartArtist {
    pub fn new(
        body_part: BodyPart,
        pixels_per_cell: f32,
        rotation_matrices: &[Matrix3<f32>; 4],
    ) -> BodyPartArtist {
        let world_offset = body_part.offset / pixels_per_cell;
        BodyPartArtist {
            world_width: (body_part.texture_width as f32) / pixels_per_cell,
            world_height: (body_part.texture_height as f32) / pixels_per_cell,
            body_part,
            offsets: [
                rotation_matrices[0] * world_offset,
                rotation_matrices[1] * world_offset,
                rotation_matrices[2] * world_offset,
                rotation_matrices[3] * world_offset,
            ],
        }
    }

    pub fn init(&self, max_avatars: usize) -> Box<dyn Iterator<Item = Command>> {
        if let Some(mask) = &self.body_part.mask {
            Box::new(self.init_masked_billboards(max_avatars, mask.mask))
        } else {
            Box::new(self.init_billboards(max_avatars))
        }
    }

    fn init_masked_billboards(
        &self,
        max_avatars: usize,
        mask: &str,
    ) -> impl Iterator<Item = Command> {
        once(create_masked_billboards(
            self.body_part.drawing_name.to_string(),
            max_avatars,
        ))
        .chain(once(update_masked_billboard_texture(
            self.body_part.drawing_name.to_string(),
            self.body_part.texture,
        )))
        .chain(once(update_masked_billboard_mask(
            self.body_part.drawing_name.to_string(),
            mask,
        )))
    }

    fn init_billboards(&self, max_avatars: usize) -> impl Iterator<Item = Command> {
        once(create_billboards(
            self.body_part.drawing_name.to_string(),
            max_avatars,
        ))
        .chain(once(update_billboard_texture(
            self.body_part.drawing_name.to_string(),
            self.body_part.texture,
        )))
    }

    pub fn draw_avatars(&self, avatars: &[ArtistAvatar]) -> Command {
        let world_coords = self.get_world_coords(avatars);

        if let Some(mask) = &self.body_part.mask {
            update_masked_billboards_vertices(
                self.body_part.drawing_name.to_string(),
                world_coords,
                self.get_colors(avatars, &mask.color_fn),
                self.world_width,
                self.world_height,
            )
        } else {
            update_billboards_vertices(
                self.body_part.drawing_name.to_string(),
                world_coords,
                self.world_width,
                self.world_height,
            )
        }
    }

    fn get_world_coords(&self, avatars: &[ArtistAvatar]) -> Vec<WorldCoord> {
        avatars
            .iter()
            .map(|avatar| self.get_world_coord(avatar))
            .collect()
    }

    fn get_world_coord(&self, avatar: &ArtistAvatar) -> WorldCoord {
        let ArtistAvatar {
            progress,
            world_coord: WorldCoord { x, y, z },
            ..
        } = avatar;
        let rotation_index = progress.rotation() as usize;
        let offset = self.offsets[rotation_index];
        WorldCoord::new(x + offset.x, y + offset.y, z + offset.z)
    }

    fn get_colors<'a>(
        &self,
        avatars: &'a [ArtistAvatar],
        color_fn: &fn(&Avatar) -> &Color,
    ) -> Vec<&'a Color> {
        avatars
            .iter()
            .map(|ArtistAvatar { avatar, .. }| (color_fn)(avatar))
            .collect()
    }
}
