pub mod drawing;
mod program;
mod shader;
pub mod texture;
mod vertex_objects;

use self::program::Program;
use self::texture::{Texture, TextureLibrary};
use self::vertex_objects::MultiVBO;
use crate::font::Font;
use commons::na;
use coords::*;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::ffi::c_void;
use std::iter::FromIterator;
use std::sync::Arc;
use transform::{Isometric, Transform};

#[derive(Debug)]
pub struct Drawing {
    name: String,
    drawing_type: DrawingType,
    indices: usize,
    max_floats_per_index: usize,
    texture: Option<String>,
    visibility_check_coord: Option<WorldCoord>,
    visible: bool,
}

impl Drawing {
    pub fn plain(name: String, floats: usize) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Plain,
            indices: 1,
            max_floats_per_index: floats,
            texture: None,
            visibility_check_coord: None,
            visible: true,
        }
    }

    pub fn billboard(name: String, floats: usize, texture: String) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Billboard,
            indices: 1,
            max_floats_per_index: floats,
            texture: Some(texture),
            visibility_check_coord: None,
            visible: true,
        }
    }

    pub fn text(
        name: String,
        floats: usize,
        font: &Font,
        visibility_check_coord: WorldCoord,
    ) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Text,
            indices: 1,
            max_floats_per_index: floats,
            texture: Some(font.texture().clone()),
            visibility_check_coord: Some(visibility_check_coord),
            visible: true,
        }
    }

    pub fn multi(name: String, indices: usize, max_floats_per_index: usize) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Plain,
            indices,
            max_floats_per_index,
            texture: None,
            visibility_check_coord: None,
            visible: true,
        }
    }
}

struct GLDrawing {
    drawing: Drawing,
    buffer: MultiVBO,
    texture: Option<Arc<Texture>>,
}

impl GLDrawing {
    pub fn new(drawing: Drawing, texture_library: &mut TextureLibrary) -> GLDrawing {
        GLDrawing {
            buffer: MultiVBO::new(
                drawing.drawing_type,
                drawing.indices,
                drawing.max_floats_per_index,
            ),
            texture: drawing
                .texture
                .as_ref()
                .map(|texture| texture_library.get_texture(texture)),
            drawing,
        }
    }

    pub fn load(&mut self, index: usize, floats: Vec<f32>) {
        self.buffer.load(index, floats);
    }

    pub fn draw(&self) {
        self.buffer.draw();
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DrawingType {
    Plain,
    Text,
    Billboard,
}

pub struct GraphicsEngine {
    programs: [Program; 3],
    viewport_size: glutin::dpi::PhysicalSize,
    transform: Transform,
    projection: Isometric,
    drawings: HashMap<String, GLDrawing>,
    texture_library: TextureLibrary,
}

impl GraphicsEngine {
    pub fn new(z_scale: f32, viewport_size: glutin::dpi::PhysicalSize) -> GraphicsEngine {
        let programs = [
            Program::from_shaders(
                DrawingType::Plain,
                include_str!("shaders/plain.vert"),
                include_str!("shaders/plain.frag"),
            ),
            Program::from_shaders(
                DrawingType::Text,
                include_str!("shaders/text.vert"),
                include_str!("shaders/text.frag"),
            ),
            Program::from_shaders(
                DrawingType::Billboard,
                include_str!("shaders/billboard.vert"),
                include_str!("shaders/billboard.frag"),
            ),
        ];

        let projection = Isometric::new(PI / 4.0, PI / 3.0);

        let transform = Transform::new(
            GLCoord3D::new(
                1.0,
                viewport_size.width as f32 / viewport_size.height as f32,
                z_scale,
            ),
            GLCoord2D::new(0.0, 0.0),
            Box::new(projection),
        );

        let mut out = GraphicsEngine {
            programs,
            viewport_size,
            transform,
            projection,
            drawings: HashMap::new(),
            texture_library: TextureLibrary::default(),
        };
        out.set_viewport_size(viewport_size);
        out.setup_open_gl();
        out
    }

    fn setup_open_gl(&mut self) {
        unsafe {
            gl::Enable(gl::BLEND);
            gl::Enable(gl::DEPTH_TEST);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
    }

    pub fn transform(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn compute_draw_order(&self, drawing_type: DrawingType) -> Vec<&GLDrawing> {
        let mut out =
            Vec::from_iter(self.drawings.values().filter(|d| {
                d.drawing.drawing_type == drawing_type && self.should_draw(&d.drawing)
            }));
        out.sort_by_key(|gl_drawing| gl_drawing.texture.as_ref().map(|texture| texture.id()));
        out
    }

    pub fn add_drawing(&mut self, drawing: Drawing) {
        self.drawings.insert(
            drawing.name.clone(),
            GLDrawing::new(drawing, &mut self.texture_library),
        );
    }

    pub fn update_drawing(&mut self, name: String, index: usize, vertices: Vec<f32>) {
        let mut gl_drawing = self.drawings.get_mut(&name).unwrap();
        gl_drawing.load(index, vertices);
        gl_drawing.drawing.visible = true;
    }

    pub fn remove_drawing(&mut self, name: &str) {
        self.drawings.remove(name);
    }

    pub fn set_drawing_visibility(&mut self, name: String, visible: bool) {
        self.drawings.get_mut(&name).unwrap().drawing.visible = visible;
    }

    fn get_pixel_to_screen(&self) -> na::Matrix2<f32> {
        na::Matrix2::new(
            2.0 / self.viewport_size.width as f32,
            0.0,
            0.0,
            2.0 / self.viewport_size.height as f32,
        )
    }

    pub fn prepare_program(&self, program: &Program) {
        match program.drawing_type {
            DrawingType::Plain => {
                program.load_matrix4("projection", self.transform.compute_transformation_matrix())
            }
            DrawingType::Text => {
                program.load_matrix4("projection", self.transform.compute_transformation_matrix());
                program.load_matrix2("pixel_to_screen", self.get_pixel_to_screen());
            }
            DrawingType::Billboard => {
                program.load_matrix4("projection", self.transform.compute_transformation_matrix());
                program.load_matrix3("world_to_screen", self.transform.get_scale_as_matrix());
            }
        }
    }

    pub fn rotate(&mut self, center: GLCoord4D, yaw: f32) {
        self.projection.yaw = (self.projection.yaw + PI * 2.0 + yaw) % (PI * 2.0);
        let proj = self.projection;

        self.transform.transform_maintaining_center(
            center,
            Box::new(move |transform| {
                transform.set_projection(Box::new(proj));
            }),
        );
    }

    pub fn draw_world(&mut self) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        self.draw(0);
    }

    pub fn draw_ui(&mut self) {
        self.draw(1);
    }

    pub fn draw_billboards(&mut self) {
        self.draw(2);
    }

    fn textures_are_different(a: &Option<Arc<Texture>>, b: &Option<Arc<Texture>>) -> bool {
        let a_id = a.as_ref().map(|texture| texture.id());
        let b_id = b.as_ref().map(|texture| texture.id());
        a_id != b_id
    }

    fn change_bound_texture(old: &Option<Arc<Texture>>, new: &Option<Arc<Texture>>) {
        unsafe {
            old.iter().for_each(|texture| texture.unbind());
            new.iter().for_each(|texture| texture.bind());
        }
    }

    fn draw(&mut self, program: usize) {
        let program = &self.programs[program];
        self.transform.compute_transformation_matrix();
        program.set_used();
        self.prepare_program(program);
        let mut current_texture: &Option<Arc<Texture>> = &None;
        for gl_drawing in self.compute_draw_order(program.drawing_type) {
            let new_texture = &gl_drawing.texture;
            if Self::textures_are_different(current_texture, new_texture) {
                Self::change_bound_texture(current_texture, new_texture);
                current_texture = new_texture;
            }
            gl_drawing.draw();
        }
        unsafe {
            current_texture.iter().for_each(|texture| texture.unbind());
        }
    }

    pub fn set_viewport_size(&mut self, viewport_size: glutin::dpi::PhysicalSize) {
        self.transform.scale(
            GLCoord4D::new(0.0, 0.0, 0.0, 1.0),
            GLCoord2D::new(
                (self.viewport_size.width as f32) / (viewport_size.width as f32),
                (self.viewport_size.height as f32) / (viewport_size.height as f32),
            ),
        );
        self.viewport_size = viewport_size;
        unsafe {
            gl::Viewport(
                0,
                0,
                viewport_size.width as i32,
                viewport_size.height as i32,
            );
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        }
    }

    fn should_draw(&self, drawing: &Drawing) -> bool {
        if !drawing.visible {
            return false;
        }
        if let Some(visibility_check_coord) = drawing.visibility_check_coord {
            self.is_visible(&visibility_check_coord)
        } else {
            true
        }
    }

    fn is_visible(&self, world_coord: &WorldCoord) -> bool {
        let gl_coord_4 = world_coord.to_gl_coord_4d(&self.transform);
        let gl_coord_2 = GLCoord2D::new(gl_coord_4.x, gl_coord_4.y);
        let physical_size = self.viewport_size;
        let buffer_coord = gl_coord_2.to_buffer_coord(physical_size);
        let z_finder = GLZFinder {};
        let actual_z = z_finder.get_z_at(buffer_coord);

        gl_coord_4.z - actual_z < 0.01
    }
}

pub struct GLZFinder {}

impl ZFinder for GLZFinder {
    fn get_z_at(&self, buffer_coordinate: BufferCoordinate) -> f32 {
        let mut buffer: Vec<f32> = vec![0.0];
        unsafe {
            gl::ReadBuffer(gl::BACK);
            gl::ReadPixels(
                buffer_coordinate.x,
                buffer_coordinate.y,
                1,
                1,
                gl::DEPTH_COMPONENT,
                gl::FLOAT,
                buffer.as_mut_ptr() as *mut c_void,
            );
        }
        2.0 * buffer[0] - 1.0
    }
}
