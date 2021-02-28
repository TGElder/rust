use std::iter::once;

use commons::na::Matrix3;
use commons::V3;
use isometric::coords::WorldCoord;
use isometric::drawing::{
    create_billboards, create_masked_billboards, update_billboard_texture,
    update_billboards_vertices, update_masked_billboard_mask, update_masked_billboard_texture,
    update_masked_billboards_vertices,
};
use isometric::Command;

use super::artist_avatar::ArtistAvatar;
use super::parameters::BodyPart;

pub struct BodyPartArtist {
    body_part: BodyPart,
    width: f32,
    height: f32,
    offsets: [V3<f32>; 4],
}

impl BodyPartArtist {
    pub fn new(
        body_part: BodyPart,
        pixels_per_cell: f32,
        rotation_matrices: &[Matrix3<f32>; 4],
    ) -> BodyPartArtist {
        let world_offset = body_part.offset / pixels_per_cell;
        BodyPartArtist {
            width: (body_part.texture_width as f32) / pixels_per_cell,
            height: (body_part.texture_height as f32) / pixels_per_cell,
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
            Box::new(
                once(create_masked_billboards(
                    self.body_part.handle.to_string(),
                    max_avatars,
                ))
                .chain(once(update_masked_billboard_texture(
                    self.body_part.handle.to_string(),
                    self.body_part.texture,
                )))
                .chain(once(update_masked_billboard_mask(
                    self.body_part.handle.to_string(),
                    mask.mask,
                ))),
            )
        } else {
            Box::new(
                once(create_billboards(
                    self.body_part.handle.to_string(),
                    max_avatars,
                ))
                .chain(once(update_billboard_texture(
                    self.body_part.handle.to_string(),
                    self.body_part.texture,
                ))),
            )
        }
    }

    pub fn draw_avatars(&self, avatars: &[ArtistAvatar]) -> Command {
        let world_coords = avatars
            .iter()
            .map(
                |ArtistAvatar {
                     progress,
                     world_coord: WorldCoord { x, y, z },
                     ..
                 }| {
                    let rotation_index = progress.rotation() as usize;
                    let offset = self.offsets[rotation_index];
                    WorldCoord::new(x + offset.x, y + offset.y, z + offset.z)
                },
            )
            .collect::<Vec<_>>();

        if let Some(mask) = &self.body_part.mask {
            let colors = avatars
                .iter()
                .map(|ArtistAvatar { avatar, .. }| (mask.color_fn)(avatar))
                .collect::<Vec<_>>();
            update_masked_billboards_vertices(
                self.body_part.handle.to_string(),
                world_coords,
                colors,
                self.width,
                self.height,
            )
        } else {
            update_billboards_vertices(
                self.body_part.handle.to_string(),
                world_coords,
                self.width,
                self.height,
            )
        }
    }
}
