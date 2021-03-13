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
    max_avatars: usize,
}

impl BoatArtist {
    pub fn new(
        light_direction: &V3<f32>,
        rotation_matrices: [Matrix3<f32>; 4],
        max_avatars: usize,
    ) -> BoatArtist {
        BoatArtist {
            draw_params: DrawBoatParams {
                width: 0.13,
                side_height: 0.04,
                bow_length: 0.06,
                mast_height: 0.4,
                base_color: Color::new(0.46875, 0.257_812_5, 0.070_312_5, 0.8),
                sail_color: Color::new(1.0, 1.0, 1.0, 1.0),
                light_direction: *light_direction,
            },
            rotation_matrices,
            max_avatars,
        }
    }

    pub fn init(&self) -> Command {
        create_boats(BOAT_DRAWING.to_string(), self.max_avatars)
    }

    pub fn draw_boats(&self, avatars: &[ArtistAvatar]) -> impl Iterator<Item = Command> {
        let mut coords = vec![];
        let mut rotations = vec![];
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
            let matrix = &self.rotation_matrices[rotation_index];
            rotations.push(matrix);
        }
        draw_boats(
            BOAT_DRAWING,
            coords,
            rotations,
            self.max_avatars,
            &self.draw_params,
        )
    }
}
