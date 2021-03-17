use super::*;
use commons::rectangle::Rectangle;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
pub struct SpriteSheet {
    frames: HashMap<String, Sprite>,
    meta: Meta,
}

impl SpriteSheet {
    pub fn load(path: &str) -> SpriteSheet {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    }

    pub fn texture_coords(&self) -> HashMap<String, Rectangle<f32>> {
        let w = self.meta.size.w as f32;
        let h = self.meta.size.h as f32;

        self.frames
            .iter()
            .map(|(name, sprite)| (name.clone(), sprite.texture_coords(&w, &h)))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct Sprite {
    frame: Frame,
}

impl Sprite {
    fn texture_coords(&self, w: &f32, h: &f32) -> Rectangle<f32> {
        Rectangle::new(
            v2(self.frame.x as f32 / w, self.frame.y as f32 / h),
            v2(
                (self.frame.x + self.frame.w) as f32 / w,
                (self.frame.y + self.frame.h) as f32 / h,
            ),
        )
    }
}

#[derive(Debug, Deserialize)]

struct Frame {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

#[derive(Debug, Deserialize)]

struct Meta {
    size: Size,
}

#[derive(Debug, Deserialize)]

struct Size {
    w: usize,
    h: usize,
}
