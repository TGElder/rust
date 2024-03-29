use std::iter::once;

use commons::na::Matrix3;
use isometric::Command;

use super::artist_avatar::ArtistAvatar;
use super::body_part_artist::BodyPartArtist;
use super::parameters::AvatarArtistParams;
use crate::artists::avatar_artist::boat_artist::BoatArtist;
use crate::artists::avatar_artist::load_artist::LoadArtist;
use crate::avatar::{Avatar, Rotation, ROTATIONS};

pub struct AvatarArtist {
    body_part_artists: Vec<BodyPartArtist>,
    boat_artist: BoatArtist,
    load_artist: LoadArtist,
    max_avatars: usize,
}

impl AvatarArtist {
    pub fn new(params: AvatarArtistParams) -> AvatarArtist {
        let rotation_matrices = get_rotation_matrices();

        AvatarArtist {
            body_part_artists: params
                .body_parts
                .iter()
                .map(|part| BodyPartArtist::new(part.clone(), &rotation_matrices))
                .collect(),
            boat_artist: BoatArtist::new(&params.boat, params.light_direction, rotation_matrices),
            load_artist: LoadArtist::new(params.load),
            max_avatars: params.max_avatars,
        }
    }

    pub fn init(&self) -> Vec<Command> {
        self.body_part_artists
            .iter()
            .flat_map(|artist| artist.init(self.max_avatars))
            .chain(once(self.boat_artist.init(self.max_avatars)))
            .chain(self.load_artist.init(self.max_avatars))
            .collect::<Vec<_>>()
    }

    pub fn draw_avatars(
        &self,
        avatars: &mut dyn Iterator<Item = &Avatar>,
        selected: Option<&String>,
        at: &u128,
    ) -> Vec<Command> {
        let avatars = avatars
            .flat_map(|avatar| ArtistAvatar::from(avatar, at))
            .filter(
                |ArtistAvatar {
                     done,
                     avatar: Avatar { name, .. },
                     ..
                 }| !done || Some(name) == selected,
            )
            .collect::<Vec<_>>();
        self.body_part_artists
            .iter()
            .map(|artist| artist.draw_avatars(&avatars))
            .chain(once(self.boat_artist.draw_boats(&avatars)))
            .chain(once(self.load_artist.draw_loads(&avatars)))
            .collect::<Vec<_>>()
    }
}

fn get_rotation_matrices() -> [Matrix3<f32>; 4] {
    let mut out: [Matrix3<f32>; 4] = [Matrix3::zeros(); 4];
    ROTATIONS
        .iter()
        .for_each(|rotation| out[*rotation as usize] = get_rotation_matrix(rotation));
    out
}

#[rustfmt::skip]
fn get_rotation_matrix(rotation: &Rotation) -> Matrix3<f32> {
    let cos = rotation.angle().cos();
    let sin = rotation.angle().sin();
    Matrix3::from_vec(vec![
        cos, sin, 0.0,
        -sin, cos, 0.0,
        0.0, 0.0, 1.0
    ])
}
