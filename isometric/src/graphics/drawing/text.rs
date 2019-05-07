use super::super::engine::DrawingType;
use super::super::vertex_objects::VBO;
use super::Drawing;
use coords::WorldCoord;
use font::Font;
use std::sync::Arc;

pub struct Text {
    vbo: VBO,
    font: Arc<Font>,
    world_coord: WorldCoord,
}

impl Drawing for Text {
    fn draw(&self) {
        unsafe {
            self.font.texture().bind();
            self.vbo.draw();
            self.font.texture().unbind();
        }
    }

    fn get_z_mod(&self) -> f32 {
        0.0
    }

    fn drawing_type(&self) -> &DrawingType {
        self.vbo.drawing_type()
    }

    fn get_visibility_check_coord(&self) -> Option<&WorldCoord> {
        Some(&self.world_coord)
    }
}

impl Text {
    #[rustfmt::skip]
    pub fn new(text: &str, world_coord: WorldCoord, font: Arc<Font>) -> Text {
        let mut vbo = VBO::new(DrawingType::Text);

        let mut vertices = vec![];

        let total_width: f32 = font.get_width(text) as f32;
        let mut xs = -total_width / 2.0;

        for character in text.chars() {
            let (top_left, bottom_right) = font.get_texture_coords(character);
            let p = world_coord;
            let (w, h) = font.get_dimensions(character);
            let (w, h) = (w as f32, h as f32);

            vertices.append(&mut vec![
                p.x, p.y, p.z, top_left.x, bottom_right.y, xs, 0.0,
                p.x, p.y, p.z, top_left.x, top_left.y, xs, h,
                p.x, p.y, p.z, bottom_right.x, top_left.y, xs + w, h,
                p.x, p.y, p.z, top_left.x, bottom_right.y, xs, 0.0,
                p.x, p.y, p.z, bottom_right.x, top_left.y, xs + w, h,
                p.x, p.y, p.z, bottom_right.x, bottom_right.y, xs + w, 0.0,
            ]);

            xs += font.get_advance(character) as f32;
        }

        vbo.load(vertices);

        Text{vbo, font, world_coord}
    }
}
