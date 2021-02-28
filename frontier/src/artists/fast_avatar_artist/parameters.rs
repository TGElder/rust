use commons::{v3, V3};
use isometric::Color;

use crate::avatar::Avatar;

pub struct AvatarArtistParams {
    pub pixels_per_cell: f32,
    pub body_parts: Vec<BodyPart>,
}

impl AvatarArtistParams {
    pub fn new() -> AvatarArtistParams {
        AvatarArtistParams {
            pixels_per_cell: 1280.0,
            body_parts: vec![
                BodyPart {
                    offset: v3(0.0, 0.0, 96.0),
                    handle: "body",
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
                    handle: "head",
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
                    handle: "left_eye",
                    texture: "resources/textures/eye.png",
                    texture_width: 16,
                    texture_height: 16,
                    mask: None,
                },
                BodyPart {
                    offset: v3(48.0, -24.0, 192.0),
                    handle: "right_eye",
                    texture: "resources/textures/eye.png",
                    texture_width: 16,
                    texture_height: 16,
                    mask: None,
                },
                BodyPart {
                    offset: v3(48.0, 50.0, 96.0),
                    handle: "left_hand",
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
                    handle: "right_hand",
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

#[derive(Clone)]
pub struct BodyPart {
    pub offset: V3<f32>,
    pub handle: &'static str,
    pub texture: &'static str,
    pub texture_width: usize,
    pub texture_height: usize,
    pub mask: Option<ColorMask>,
}

#[derive(Clone)]
pub struct ColorMask {
    pub mask: &'static str,
    pub color_fn: fn(&Avatar) -> &Color,
}
