use commons::{v3, V3};

use crate::artists::fast_avatar_artist::boat_artist::BoatArtistParams;
use crate::artists::fast_avatar_artist::body_part_artist::{BodyPart, ColorMask};

pub struct AvatarArtistParams {
    pub boat: BoatArtistParams,
    pub max_avatars: usize,
    pub light_direction: V3<f32>,
    pub pixels_per_cell: f32,
    pub body_parts: Vec<BodyPart>,
}

impl Default for AvatarArtistParams {
    fn default() -> Self {
        AvatarArtistParams {
            boat: BoatArtistParams::default(),
            max_avatars: 0,
            light_direction: v3(1.0, 1.0, 1.0),
            pixels_per_cell: 1280.0,
            body_parts: vec![
                BodyPart {
                    offset: v3(0.0, 0.0, 96.0),
                    drawing_name: "body",
                    texture: "resources/textures/body.png",
                    texture_width: 128,
                    texture_height: 192,
                    mask: Some(ColorMask {
                        mask: "resources/textures/body.png",
                        color_fn: |avatar| &avatar.color,
                    }),
                },
                BodyPart {
                    offset: v3(12.0, 0.0, 192.0),
                    drawing_name: "head",
                    texture: "resources/textures/head.png",
                    texture_width: 96,
                    texture_height: 96,
                    mask: Some(ColorMask {
                        mask: "resources/textures/head.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
                BodyPart {
                    offset: v3(48.0, 24.0, 192.0),
                    drawing_name: "left_eye",
                    texture: "resources/textures/eye.png",
                    texture_width: 16,
                    texture_height: 16,
                    mask: None,
                },
                BodyPart {
                    offset: v3(48.0, -24.0, 192.0),
                    drawing_name: "right_eye",
                    texture: "resources/textures/eye.png",
                    texture_width: 16,
                    texture_height: 16,
                    mask: None,
                },
                BodyPart {
                    offset: v3(48.0, 50.0, 96.0),
                    drawing_name: "left_hand",
                    texture: "resources/textures/hand.png",
                    texture_width: 32,
                    texture_height: 32,
                    mask: Some(ColorMask {
                        mask: "resources/textures/hand.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
                BodyPart {
                    offset: v3(48.0, -50.0, 96.0),
                    drawing_name: "right_hand",
                    texture: "resources/textures/hand.png",
                    texture_width: 32,
                    texture_height: 32,
                    mask: Some(ColorMask {
                        mask: "resources/textures/hand.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
            ],
        }
    }
}
