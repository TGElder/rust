use std::iter::once;

use commons::na::Matrix3;
use commons::{v3, V3};
use isometric::coords::WorldCoord;
use isometric::drawing::{create_billboards, update_billboard_texture, update_billboards_vertices};
use isometric::{Color, Command};

use crate::artists::AvatarArtistParams;
use crate::avatar::{Avatar, Progress, Rotation};

pub struct FastAvatarArtist {
    body_part_artists: Vec<BodyPartArtist>,
}

struct BodyPartArtist {
    body_part: BodyPart,
    width: f32,
    height: f32,
    offsets: [V3<f32>; 4],
}

impl BodyPartArtist {
    fn new(
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

    fn init(&self) -> impl Iterator<Item = Command> {
        once(create_billboards(self.body_part.handle.clone(), 1024)).chain(once(
            update_billboard_texture(self.body_part.handle.clone(), &self.body_part.texture),
        ))
    }

    fn draw_avatars(&self, avatars: &[ArtistAvatar]) -> Command {
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

        update_billboards_vertices(
            self.body_part.handle.clone(),
            world_coords,
            self.width,
            self.height,
        )
    }
}

impl FastAvatarArtist {
    pub fn new(params: &AvatarArtistParams) -> FastAvatarArtist {
        let rotation_matrices: [Matrix3<f32>; 4] = [
            get_rotation_matrix(&Rotation::Left),
            get_rotation_matrix(&Rotation::Up),
            get_rotation_matrix(&Rotation::Right),
            get_rotation_matrix(&Rotation::Down),
        ];
        FastAvatarArtist {
            body_part_artists: vec![
                // BodyPart {
                //     offset: v3(0.0, 0.0, 96.0),
                //     handle: "body".to_string(),
                //     texture: "resources/textures/body.png".to_string(),
                //     texture_width: 128,
                //     texture_height: 192,
                //     mask: Some(ColorMask {
                //         mask: "resources/textures/body.png".to_string(),
                //         color: AvatarColor::Base,
                //     }),
                // },
                // BodyPart {
                //     offset: v3(12.0, 0.0, 192.0),
                //     handle: "head".to_string(),
                //     texture: "resources/textures/head.png".to_string(),
                //     texture_width: 96,
                //     texture_height: 96,
                //     mask: Some(ColorMask {
                //         mask: "resources/textures/head.png".to_string(),
                //         color: AvatarColor::Skin,
                //     }),
                // },
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
            ]
            .into_iter()
            .map(|part| BodyPartArtist::new(part, params.pixels_per_cell, &rotation_matrices))
            .collect(),
        }
    }

    pub fn init(&self) -> Vec<Command> {
        self.body_part_artists
            .iter()
            .flat_map(|artist| artist.init())
            .collect::<Vec<_>>()
    }

    pub fn draw_avatars(
        &self,
        avatars: &mut dyn Iterator<Item = &Avatar>,
        at: &u128,
    ) -> Vec<Command> {
        let avatars = avatars
            .flat_map(|avatar| ArtistAvatar::from(avatar, at))
            .collect::<Vec<_>>();
        self.body_part_artists
            .iter()
            .map(|artist| artist.draw_avatars(&avatars))
            .collect::<Vec<_>>()
    }
}

fn get_rotation_matrix(rotation: &Rotation) -> Matrix3<f32> {
    let cos = rotation.angle().cos();
    let sin = rotation.angle().sin();
    Matrix3::from_vec(vec![cos, sin, 0.0, -sin, cos, 0.0, 0.0, 0.0, 1.0])
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

struct ArtistAvatar<'a> {
    avatar: &'a Avatar,
    progress: Progress<'a>,
    world_coord: WorldCoord,
}

impl<'a> ArtistAvatar<'a> {
    fn from(avatar: &'a Avatar, at: &u128) -> Option<ArtistAvatar<'a>> {
        let progress = avatar.journey.as_ref()?.progress_at(at);
        let world_coord = progress.world_coord_at(at);
        Some(ArtistAvatar {
            avatar,
            progress,
            world_coord,
        })
    }
}
