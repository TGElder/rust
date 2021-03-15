use commons::na::Matrix3;
use commons::{v3, V3};
use isometric::drawing::{
    create_plain, get_colored_vertices_from_square_both_sides,
    get_colored_vertices_from_triangle_both_sides, offset_plain_floats, AngleSquareColoring,
    AngleTriangleColoring,
};
use isometric::{Color, Command};

use crate::artists::fast_avatar_artist::artist_avatar::ArtistAvatar;
use crate::avatar::Vehicle;

const BOAT_DRAWING: &str = "boats";
const BOAT_FLOATS: usize = 540;

pub struct BoatArtist {
    boat_floats: Vec<Vec<f32>>,
}

pub struct BoatArtistParams {
    pub width: f32,
    pub side_height: f32,
    pub bow_length: f32,
    pub mast_height: f32,
    pub base_color: Color,
    pub sail_color: Color,
}

impl Default for BoatArtistParams {
    fn default() -> Self {
        BoatArtistParams {
            width: 0.13,
            side_height: 0.04,
            bow_length: 0.06,
            mast_height: 0.4,
            base_color: Color::new(0.46875, 0.257_812_5, 0.070_312_5, 0.8),
            sail_color: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

impl BoatArtist {
    pub fn new(
        params: &BoatArtistParams,
        light_direction: V3<f32>,
        rotation_matrices: [Matrix3<f32>; 4],
    ) -> BoatArtist {
        BoatArtist {
            boat_floats: rotation_matrices
                .iter()
                .map(|rotation| boat_floats(&params, light_direction, rotation))
                .collect(),
        }
    }

    pub fn init(&self, max_avatars: usize) -> Command {
        create_plain(BOAT_DRAWING.to_string(), BOAT_FLOATS * max_avatars)
    }

    pub fn draw_boats(&self, avatars: &[ArtistAvatar]) -> Command {
        let avatars = avatars_with_boats(avatars);
        let mut floats = vec![0.0; BOAT_FLOATS * avatars.len()];
        avatars.into_iter().enumerate().for_each(|(i, avatar)| {
            self.draw_boat(
                avatar,
                &mut floats[(i * BOAT_FLOATS)..((i + 1) * BOAT_FLOATS)],
            )
        });
        Command::UpdateVertices {
            name: BOAT_DRAWING.to_string(),
            floats,
            index: 0,
        }
    }

    fn draw_boat<'a>(&'a self, avatar: &'a ArtistAvatar, target: &mut [f32]) {
        let ArtistAvatar {
            progress,
            world_coord,
            ..
        } = avatar;

        let rotation_index = progress.rotation() as usize;
        offset_plain_floats(&self.boat_floats[rotation_index], target, world_coord)
    }
}

fn avatars_with_boats<'a>(avatars: &'a [ArtistAvatar]) -> Vec<&'a ArtistAvatar<'a>> {
    avatars
        .iter()
        .filter(|avatar| avatar_has_boat(avatar))
        .collect()
}

fn avatar_has_boat(avatar: &ArtistAvatar) -> bool {
    let ArtistAvatar { progress, .. } = avatar;
    progress.vehicle() == Vehicle::Boat
}

pub fn boat_floats(
    p: &BoatArtistParams,
    light_direction: V3<f32>,
    rotation: &Matrix3<f32>,
) -> Vec<f32> {
    let triangle_coloring = AngleTriangleColoring::new(p.base_color, light_direction);
    let square_coloring = AngleSquareColoring::new(p.base_color, light_direction);

    let offset = v3(0.0, 0.0, 0.01);

    let width_2 = p.width / 2.0;

    let al = (rotation * v3(-width_2, -width_2, 0.0)) + offset;
    let bl = (rotation * v3(2.0 * width_2, -width_2, 0.0)) + offset;
    let cl = (rotation * v3(2.0 * width_2, width_2, 0.0)) + offset;
    let dl = (rotation * v3(-width_2, width_2, 0.0)) + offset;
    let ah = (rotation * v3(-width_2, -width_2, p.side_height)) + offset;
    let bh = (rotation * v3(2.0 * width_2, -width_2, p.side_height)) + offset;
    let ch = (rotation * v3(2.0 * width_2, width_2, p.side_height)) + offset;
    let dh = (rotation * v3(-width_2, width_2, p.side_height)) + offset;

    let el = (rotation * v3((2.0 * width_2) + p.bow_length, 0.0, 0.0)) + offset;
    let eh = (rotation * v3((2.0 * width_2) + p.bow_length, 0.0, p.side_height)) + offset;

    let sa = (rotation * v3(2.0 * width_2 + (p.bow_length / 2.0), 0.0, p.side_height)) + offset;
    let sb = (rotation * v3(2.0 * width_2 + (p.bow_length / 2.0), 0.0, p.mast_height)) + offset;
    let sc = (rotation * v3(-0.3 * width_2, 1.5 * width_2, p.side_height)) + offset;

    let mut floats = vec![];
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[al, bl, bh, ah],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[dl, al, ah, dh],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[el, bl, bh, eh],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[dl, cl, ch, dh],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[cl, el, eh, ch],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[al, bl, cl, dl],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle_both_sides(
        &[bl, el, cl],
        &triangle_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle_both_sides(
        &[bh, eh, ch],
        &triangle_coloring,
    ));

    let sail_coloring = AngleTriangleColoring::new(p.sail_color, light_direction);

    floats.append(&mut get_colored_vertices_from_triangle_both_sides(
        &[sa, sb, sc],
        &sail_coloring,
    ));

    floats
}
