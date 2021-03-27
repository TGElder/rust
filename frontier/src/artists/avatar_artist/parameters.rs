use commons::{v3, V3};

use crate::artists::avatar_artist::boat_artist::BoatArtistParams;
use crate::artists::avatar_artist::body_part_artist::{BodyPart, ColorMask};
use crate::artists::avatar_artist::load_artist::LoadArtistParams;

pub struct AvatarArtistParams {
    pub boat: BoatArtistParams,
    pub load: LoadArtistParams,
    pub max_avatars: usize,
    pub light_direction: V3<f32>,
    pub pixels_per_cell: f32,
    pub body_parts: Vec<BodyPart>,
}

impl Default for AvatarArtistParams {
    fn default() -> Self {
        AvatarArtistParams {
            boat: BoatArtistParams::default(),
            load: LoadArtistParams::default(),
            max_avatars: 0,
            light_direction: v3(1.0, 1.0, 1.0),
            pixels_per_cell: 512.0,
            body_parts: vec![
                BodyPart {
                    offset: v3(0.0, 0.0, 39.0),
                    drawing_name: "body",
                    texture: "resources/textures/body.png",
                    texture_width: 52,
                    texture_height: 78,
                    mask: Some(ColorMask {
                        mask: "resources/textures/body.png",
                        color_fn: |avatar| &avatar.color,
                    }),
                },
                BodyPart {
                    offset: v3(5.0, 0.0, 78.0),
                    drawing_name: "head",
                    texture: "resources/textures/head.png",
                    texture_width: 39,
                    texture_height: 39,
                    mask: Some(ColorMask {
                        mask: "resources/textures/head.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
                BodyPart {
                    offset: v3(19.0, 10.0, 78.0),
                    drawing_name: "left_eye",
                    texture: "resources/textures/eye.png",
                    texture_width: 7,
                    texture_height: 7,
                    mask: None,
                },
                BodyPart {
                    offset: v3(19.0, -10.0, 78.0),
                    drawing_name: "right_eye",
                    texture: "resources/textures/eye.png",
                    texture_width: 7,
                    texture_height: 7,
                    mask: None,
                },
                BodyPart {
                    offset: v3(19.0, 20.0, 39.0),
                    drawing_name: "left_hand",
                    texture: "resources/textures/hand.png",
                    texture_width: 13,
                    texture_height: 13,
                    mask: Some(ColorMask {
                        mask: "resources/textures/hand.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
                BodyPart {
                    offset: v3(19.0, -20.0, 39.0),
                    drawing_name: "right_hand",
                    texture: "resources/textures/hand.png",
                    texture_width: 13,
                    texture_height: 13,
                    mask: Some(ColorMask {
                        mask: "resources/textures/hand.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
            ],
        }
    }
}
