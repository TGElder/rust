use commons::na::Matrix3;
use commons::V3;
use isometric::drawing::{create_boats, draw_boats, DrawBoatParams};
use isometric::{Color, Command};

use crate::artists::fast_avatar_artist::artist_avatar::ArtistAvatar;
use crate::avatar::Vehicle;

const BOAT_DRAWING: &str = "boats";

pub struct BoatArtist {
    draw_params: DrawBoatParams,
    rotation_matrices: [Matrix3<f32>; 4],
}

impl BoatArtist {
    pub fn new(light_direction: V3<f32>, rotation_matrices: [Matrix3<f32>; 4]) -> BoatArtist {
        BoatArtist {
            draw_params: DrawBoatParams {
                width: 0.13,
                side_height: 0.04,
                bow_length: 0.06,
                mast_height: 0.4,
                base_color: Color::new(0.46875, 0.257_812_5, 0.070_312_5, 0.8),
                sail_color: Color::new(1.0, 1.0, 1.0, 1.0),
                light_direction,
            },
            rotation_matrices,
        }
    }

    pub fn init(&self, max_avatars: usize) -> Command {
        create_boats(BOAT_DRAWING.to_string(), max_avatars)
    }

    pub fn draw_boats(&self, avatars: &[ArtistAvatar]) -> Command {
        let mut coords = Vec::with_capacity(avatars.len());
        let mut rotation_matrices = Vec::with_capacity(avatars.len());
        for ArtistAvatar {
            progress,
            world_coord,
            ..
        } in avatars
        {
            if progress.vehicle() != Vehicle::Boat {
                continue;
            }

            coords.push(*world_coord);

            let rotation_index = progress.rotation() as usize;
            let rotation_matrix = &self.rotation_matrices[rotation_index];
            rotation_matrices.push(rotation_matrix);
        }
        draw_boats(BOAT_DRAWING, coords, rotation_matrices, &self.draw_params)
    }
}
