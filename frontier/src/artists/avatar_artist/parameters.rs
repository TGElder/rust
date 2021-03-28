use commons::{v3, V3};

use crate::artists::avatar_artist::boat_artist::BoatArtistParams;
use crate::artists::avatar_artist::body_part_artist::{BodyPart, ColorMask};
use crate::artists::avatar_artist::load_artist::LoadArtistParams;

pub struct AvatarArtistParams {
    pub boat: BoatArtistParams,
    pub load: LoadArtistParams,
    pub max_avatars: usize,
    pub light_direction: V3<f32>,
    pub body_parts: Vec<BodyPart>,
}

impl Default for AvatarArtistParams {
    fn default() -> Self {
        AvatarArtistParams {
            boat: BoatArtistParams::default(),
            load: LoadArtistParams::default(),
            max_avatars: 0,
            light_direction: v3(1.0, 1.0, 1.0),
            body_parts: vec![
                BodyPart {
                    offset: v3(0.0, 0.0, 36.0) / 512.0,
                    drawing_name: "body",
                    texture: "resources/textures/body.png",
                    width: 48.0 / 512.0,
                    height: 72.0 / 512.0,
                    mask: Some(ColorMask {
                        mask: "resources/textures/body.png",
                        color_fn: |avatar| &avatar.color,
                    }),
                },
                BodyPart {
                    offset: v3(4.0, 0.0, 72.0) / 512.0,
                    drawing_name: "head",
                    texture: "resources/textures/head.png",
                    width: 36.0 / 512.0,
                    height: 36.0 / 512.0,
                    mask: Some(ColorMask {
                        mask: "resources/textures/head.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
                BodyPart {
                    offset: v3(18.0, 9.0, 72.0) / 512.0,
                    drawing_name: "left_eye",
                    texture: "resources/textures/eye.png",
                    width: 6.0 / 512.0,
                    height: 6.0 / 512.0,
                    mask: None,
                },
                BodyPart {
                    offset: v3(18.0, -9.0, 72.0) / 512.0,
                    drawing_name: "right_eye",
                    texture: "resources/textures/eye.png",
                    width: 6.0 / 512.0,
                    height: 6.0 / 512.0,
                    mask: None,
                },
                BodyPart {
                    offset: v3(18.0, 19.0, 36.0) / 512.0,
                    drawing_name: "left_hand",
                    texture: "resources/textures/hand.png",
                    width: 12.0 / 512.0,
                    height: 12.0 / 512.0,
                    mask: Some(ColorMask {
                        mask: "resources/textures/hand.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
                BodyPart {
                    offset: v3(18.0, -19.0, 36.0) / 512.0,
                    drawing_name: "right_hand",
                    texture: "resources/textures/hand.png",
                    width: 12.0 / 512.0,
                    height: 12.0 / 512.0,
                    mask: Some(ColorMask {
                        mask: "resources/textures/hand.png",
                        color_fn: |avatar| &avatar.skin_color,
                    }),
                },
            ],
        }
    }
}
