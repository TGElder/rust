use super::super::engine::DrawingType;
use super::super::texture::Texture;
use super::super::vertex_objects::VBO;
use super::Drawing;
use coords::WorldCoord;
use std::sync::Arc;

pub struct Billboard {
    vbo: VBO,
    texture: Arc<Texture>,
}

impl Drawing for Billboard {
    fn draw(&self) {
        unsafe {
            self.texture.bind();
            self.vbo.draw();
            self.texture.unbind();
        }
    }

    fn get_z_mod(&self) -> f32 {
        0.0
    }

    fn drawing_type(&self) -> &DrawingType {
        self.vbo.drawing_type()
    }

    fn get_visibility_check_coord(&self) -> Option<&WorldCoord> {
        None
    }
}

impl Billboard {
    #[rustfmt::skip]
    pub fn new(world_coord: WorldCoord, width: f32, height: f32, texture: Arc<Texture>) -> Billboard {
        let mut vbo = VBO::new(DrawingType::Billboard);

        let p = world_coord;

        let left = -width / 2.0;
        let right = -left;
        let top = -height / 2.0;
        let bottom = -top;

        let vertices = vec![
            p.x, p.y, p.z, 0.0, 1.0, left, top,
            p.x, p.y, p.z, 0.0, 0.0, left, bottom,
            p.x, p.y, p.z, 1.0, 0.0, right, bottom,
            p.x, p.y, p.z, 0.0, 1.0, left, top,
            p.x, p.y, p.z, 1.0, 0.0, right, bottom,
            p.x, p.y, p.z, 1.0, 1.0, right, top,
        ];

        vbo.load(vertices);

        Billboard{vbo, texture}
    }
}
