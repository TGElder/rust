use commons::na::Matrix3;
use commons::v3;
use isometric::Command;

use super::artist_avatar::ArtistAvatar;
use super::body_part_artist::{AvatarColor, BodyPart, BodyPartArtist, ColorMask};
use crate::artists::AvatarArtistParams;
use crate::avatar::{Avatar, Rotation};

pub struct FastAvatarArtist {
    body_part_artists: Vec<BodyPartArtist>,
    max_avatars: usize,
}

impl FastAvatarArtist {
    pub fn new(params: &AvatarArtistParams, max_avatars: usize) -> FastAvatarArtist {
        let rotation_matrices: [Matrix3<f32>; 4] = [
            get_rotation_matrix(&Rotation::Left),
            get_rotation_matrix(&Rotation::Up),
            get_rotation_matrix(&Rotation::Right),
            get_rotation_matrix(&Rotation::Down),
        ];
        FastAvatarArtist {
            body_part_artists: vec![
                BodyPart {
                    offset: v3(0.0, 0.0, 96.0),
                    handle: "body".to_string(),
                    texture: "resources/textures/body.png".to_string(),
                    texture_width: 128,
                    texture_height: 192,
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
            ]
            .into_iter()
            .map(|part| BodyPartArtist::new(part, params.pixels_per_cell, &rotation_matrices))
            .collect(),
            max_avatars,
        }
    }

    pub fn init(&self) -> Vec<Command> {
        self.body_part_artists
            .iter()
            .flat_map(|artist| artist.init(self.max_avatars))
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
