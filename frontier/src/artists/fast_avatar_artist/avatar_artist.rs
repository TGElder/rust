use commons::na::Matrix3;
use isometric::Command;

use super::artist_avatar::ArtistAvatar;
use super::body_part_artist::BodyPartArtist;
use super::parameters::AvatarArtistParams;
use crate::avatar::{Avatar, Rotation};

pub struct AvatarArtist {
    body_part_artists: Vec<BodyPartArtist>,
    max_avatars: usize,
}

impl AvatarArtist {
    pub fn new(params: &AvatarArtistParams, max_avatars: usize) -> AvatarArtist {
        let rotation_matrices: [Matrix3<f32>; 4] = [
            get_rotation_matrix(&Rotation::Left),
            get_rotation_matrix(&Rotation::Up),
            get_rotation_matrix(&Rotation::Right),
            get_rotation_matrix(&Rotation::Down),
        ];
        AvatarArtist {
            body_part_artists: params
                .body_parts
                .iter()
                .map(|part| {
                    BodyPartArtist::new(part.clone(), params.pixels_per_cell, &rotation_matrices)
                })
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
