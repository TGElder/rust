use commons::v3;

use super::body_part_artist::{AvatarColor, BodyPart, ColorMask};

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
            ],
        }
    }
}
