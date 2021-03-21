pub mod drawing;
mod frame_buffer;
mod label_visibility_check;
mod pixel_buffer;
mod program;
mod shader;
pub mod texture;
mod vertex_objects;

use crate::graphics::frame_buffer::FrameBuffer;
use crate::graphics::pixel_buffer::PixelBuffer;

use self::label_visibility_check::{LabelVisibilityCheck, LabelVisibilityChecker};
use self::program::Program;
use self::texture::{Texture, TextureLibrary};
use self::vertex_objects::MultiVBO;
use commons::log::{error, debug};
use commons::na;
use coords::*;
use glutin::dpi::PhysicalSize;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::ffi::c_void;
use std::sync::Arc;
use std::time::Instant;
use transform::{Isometric, Transform};

pub struct GraphicsEngine {
    programs: [Program; 6],
    viewport_size: PhysicalSize<u32>,
    label_padding: f32,
    pub transform: Transform,
    projection: Isometric,
    drawings: HashMap<String, GLDrawing>,
    texture_library: TextureLibrary,
    frame_buffers: [FrameBuffer; 4],
    current_frame_buffer: usize,
    pub z_finder: GLZFinder,
}

pub struct GraphicsEngineParameters {
    pub z_scale: f32,
    pub viewport_size: PhysicalSize<u32>,
    pub label_padding: f32,
}

impl GraphicsEngine {
    pub fn new(params: GraphicsEngineParameters) -> GraphicsEngine {
        let programs = [
            Program::from_shaders(
                DrawingType::Plain,
                include_str!("shaders/plain.vert"),
                include_str!("shaders/plain.frag"),
            ),
            Program::from_shaders(
                DrawingType::Label,
                include_str!("shaders/text.vert"),
                include_str!("shaders/text.frag"),
            ),
            Program::from_shaders(
                DrawingType::Billboard,
                include_str!("shaders/billboard.vert"),
                include_str!("shaders/billboard.frag"),
            ),
            Program::from_shaders(
                DrawingType::MaskedBillboard,
                include_str!("shaders/masked_billboard.vert"),
                include_str!("shaders/masked_billboard.frag"),
            ),
            Program::from_shaders(
                DrawingType::Textured,
                include_str!("shaders/textured.vert"),
                include_str!("shaders/textured.frag"),
            ),
            Program::from_shaders(
                DrawingType::FullScreenQuad,
                include_str!("shaders/full_screen_quad.vert"),
                include_str!("shaders/full_screen_quad.frag"),
            ),
        ];

        let projection = Isometric::new(PI / 4.0, PI / 3.0);

        let transform = Transform::new(
            GLCoord3D::new(
                1.0,
                params.viewport_size.width as f32 / params.viewport_size.height as f32,
                params.z_scale,
            ),
            GLCoord2D::new(0.0, 0.0),
            Box::new(projection),
        );

        let mut out = GraphicsEngine {
            programs,
            viewport_size: params.viewport_size,
            label_padding: params.label_padding,
            transform,
            projection,
            drawings: HashMap::new(),
            texture_library: TextureLibrary::default(),
            frame_buffers: [FrameBuffer::new(
                params.viewport_size.width as i32,
                params.viewport_size.height as i32,
            ), FrameBuffer::new(
                params.viewport_size.width as i32,
                params.viewport_size.height as i32,
            ),FrameBuffer::new(
                params.viewport_size.width as i32,
                params.viewport_size.height as i32,
            ),FrameBuffer::new(
                params.viewport_size.width as i32,
                params.viewport_size.height as i32,
            )],
            current_frame_buffer: 0,
            z_finder: GLZFinder::new(params.viewport_size.width as usize, params.viewport_size.height as usize),
        };
        out.set_viewport_size(params.viewport_size);
        out.setup_open_gl();
        out
    }

    fn setup_open_gl(&mut self) {
        unsafe {
            gl::Enable(gl::BLEND);
            gl::Enable(gl::DEPTH_TEST);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
        }
    }

    pub fn setup_frame_buffer(&mut self) {
        self.frame_buffers = [FrameBuffer::new(
            self.viewport_size.width as i32,
            self.viewport_size.height as i32,
        ), FrameBuffer::new(
            self.viewport_size.width as i32,
            self.viewport_size.height as i32,
        ), FrameBuffer::new(
            self.viewport_size.width as i32,
            self.viewport_size.height as i32,
        ), FrameBuffer::new(
            self.viewport_size.width as i32,
            self.viewport_size.height as i32,
        )]
    }

    pub fn transform(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn compute_draw_order(&self, drawing_type: DrawingType) -> Vec<&GLDrawing> {
        let mut out: Vec<&GLDrawing> = self
            .drawings
            .values()
            .filter(|d| d.drawing.drawing_type == drawing_type)
            .collect();
        out.sort_by_key(|gl_drawing| {
            (
                gl_drawing.drawing.draw_order,
                gl_drawing.texture.as_ref().map(|texture| texture.id()),
            )
        });
        out
    }

    pub fn add_drawing(&mut self, drawing: Drawing) {
        self.drawings
            .insert(drawing.name.clone(), GLDrawing::new(drawing));
    }

    pub fn update_vertices(&mut self, name: String, index: usize, vertices: Vec<f32>) {
        let mut gl_drawing = self.drawings.get_mut(&name).unwrap();
        gl_drawing.load(index, vertices);
        gl_drawing.drawing.visible = true;
    }

    pub fn update_texture(&mut self, name: String, texture: Option<String>) {
        let gl_drawing = self.drawings.get_mut(&name).unwrap();
        let texture_library = &mut self.texture_library;
        gl_drawing.texture = texture.map(|texture| texture_library.get_texture(&texture))
    }

    pub fn update_mask(&mut self, name: String, texture: Option<String>) {
        let gl_drawing = self.drawings.get_mut(&name).unwrap();
        let texture_library = &mut self.texture_library;
        gl_drawing.mask = texture.map(|texture| texture_library.get_texture(&texture))
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
            DrawingType::Label => {
                program.load_matrix4("projection", self.transform.compute_transformation_matrix());
                program.load_matrix2("pixel_to_screen", self.get_pixel_to_screen());
            }
            DrawingType::Billboard => {
                program.load_matrix4("projection", self.transform.compute_transformation_matrix());
                program.load_matrix3("world_to_screen", self.transform.get_scale_as_matrix());
                program.link_texture_slot_to_variable(0, "ourTexture");
            }
            DrawingType::MaskedBillboard => {
                program.load_matrix4("projection", self.transform.compute_transformation_matrix());
                program.load_matrix3("world_to_screen", self.transform.get_scale_as_matrix());
                program.link_texture_slot_to_variable(0, "ourTexture");
                program.link_texture_slot_to_variable(1, "ourMask");
            }
            DrawingType::Textured => {
                program.load_matrix4("projection", self.transform.compute_transformation_matrix());
                program.link_texture_slot_to_variable(0, "ourTexture");
            }
            _ => (),
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

    pub fn clear(&self) {
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }

    pub fn draw_world(&mut self) {
        
        self.draw(0);
    }

    pub fn draw_ui(&mut self) {
        self.draw(1);
    }

    pub fn draw_billboards(&mut self) {
        self.draw(2);
        self.draw(3);
    }

    pub fn draw_textured(&mut self) {
        self.draw(4);
    }

    pub fn begin_drawing(&mut self) {
        self.current_frame_buffer = (self.current_frame_buffer + 1)%3;
        self.frame_buffers[self.current_frame_buffer].begin_drawing();
    }

    pub fn copy_to_back_buffer(&mut self) {
        self.frame_buffers[self.current_frame_buffer].copy_to_back_buffer(&self.programs[5]);
    }

    fn textures_are_different(a: &Option<Arc<Texture>>, b: &Option<Arc<Texture>>) -> bool {
        let a_id = a.as_ref().map(|texture| texture.id());
        let b_id = b.as_ref().map(|texture| texture.id());
        a_id != b_id
    }

    fn change_bound_texture(slot: u32, old: &Option<Arc<Texture>>, new: &Option<Arc<Texture>>) {
        unsafe {
            old.iter().for_each(|texture| texture.unbind(slot));
            new.iter().for_each(|texture| texture.bind(slot));
        }
    }

    fn draw(&mut self, program: usize) {
        let program = &self.programs[program];
        self.transform.compute_transformation_matrix();
        program.set_used();
        self.prepare_program(program);
        let mut current_texture: &Option<Arc<Texture>> = &None;
        let mut current_mask: &Option<Arc<Texture>> = &None;
        let mut label_visibility_checker = LabelVisibilityChecker::new(self);
        for gl_drawing in self.compute_draw_order(program.drawing_type) {
            if !self.should_draw(&gl_drawing.drawing, &mut label_visibility_checker) {
                continue;
            }
            let new_texture = &gl_drawing.texture;
            if Self::textures_are_different(current_texture, new_texture) {
                Self::change_bound_texture(0, current_texture, new_texture);
                current_texture = new_texture;
            }
            let new_mask = &gl_drawing.mask;
            if Self::textures_are_different(current_mask, new_mask) {
                Self::change_bound_texture(1, current_mask, new_mask);
                current_mask = new_mask;
            }
            gl_drawing.draw();
        }
        unsafe {
            current_texture.iter().for_each(|texture| texture.unbind(0));
            current_mask.iter().for_each(|texture| texture.unbind(1));
        }
    }

    pub fn set_viewport_size(&mut self, viewport_size: PhysicalSize<u32>) {
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

    fn should_draw(
        &self,
        drawing: &Drawing,
        label_visibility_checker: &mut LabelVisibilityChecker,
    ) -> bool {
        if !drawing.visible {
            return false;
        }
        if let Some(label_visibility_check) = &drawing.label_visibility_check {
            label_visibility_checker.is_visible(&label_visibility_check)
        } else {
            true
        }
    }

    pub fn update_z(&mut self) {
        unsafe {
            self.frame_buffers[self.current_frame_buffer].depth_buffer.bind();
            self.z_finder.read();
            self.frame_buffers[self.current_frame_buffer].depth_buffer.unbind();
        }

    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DrawingType {
    Plain,
    Label,
    Billboard,
    MaskedBillboard,
    Textured,
    FullScreenQuad,
}

#[derive(Debug)]
pub struct Drawing {
    name: String,
    drawing_type: DrawingType,
    indices: usize,
    max_floats_per_index: usize,
    visible: bool,
    label_visibility_check: Option<LabelVisibilityCheck>,
    draw_order: i32,
}

impl Drawing {
    pub fn plain(name: String, floats: usize) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Plain,
            indices: 1,
            max_floats_per_index: floats,
            visible: true,
            label_visibility_check: None,
            draw_order: 0,
        }
    }

    pub fn textured(name: String, floats: usize) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Textured,
            indices: 1,
            max_floats_per_index: floats,
            visible: true,
            label_visibility_check: None,
            draw_order: 0,
        }
    }

    pub fn billboard(name: String, floats: usize) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Billboard,
            indices: 1,
            max_floats_per_index: floats,
            visible: true,
            label_visibility_check: None,
            draw_order: 0,
        }
    }

    pub fn masked_billboard(name: String, floats: usize) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::MaskedBillboard,
            indices: 1,
            max_floats_per_index: floats,
            visible: true,
            label_visibility_check: None,
            draw_order: 0,
        }
    }

    pub fn label(
        name: String,
        floats: usize,
        label_visibility_check: LabelVisibilityCheck,
        draw_order: i32,
    ) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Label,
            indices: 1,
            max_floats_per_index: floats,
            visible: true,
            label_visibility_check: Some(label_visibility_check),
            draw_order,
        }
    }

    pub fn multi(name: String, indices: usize, max_floats_per_index: usize) -> Drawing {
        Drawing {
            name,
            drawing_type: DrawingType::Plain,
            indices,
            max_floats_per_index,
            visible: true,
            label_visibility_check: None,
            draw_order: 0,
        }
    }
}

struct GLDrawing {
    drawing: Drawing,
    buffer: MultiVBO,
    texture: Option<Arc<Texture>>,
    mask: Option<Arc<Texture>>,
}

impl GLDrawing {
    pub fn new(drawing: Drawing) -> GLDrawing {
        GLDrawing {
            buffer: MultiVBO::new(
                drawing.drawing_type,
                drawing.indices,
                drawing.max_floats_per_index,
            ),
            drawing,
            texture: None,
            mask: None,
        }
    }

    pub fn load(&mut self, index: usize, floats: Vec<f32>) {
        self.buffer.load(index, floats);
    }

    pub fn draw(&self) {
        self.buffer.draw();
    }
}

pub struct GLZFinder {
    pixel_buffers: [PixelBuffer; 4],
    current: usize,
    width: usize,
    height: usize,
}

impl GLZFinder {
    pub fn new(width: usize, height: usize) -> GLZFinder {
        GLZFinder {
            pixel_buffers: [PixelBuffer::new(width, height), PixelBuffer::new(width, height), PixelBuffer::new(width, height), PixelBuffer::new(width, height)],
            current: 0,
            width,
            height,
        }
    }

    pub fn read(&mut self) {
        self.current = (self.current + 3) % 4;
        unsafe {
            self.pixel_buffers[(self.current + 3) % 4].bind();
            loop {
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    error!("{}", error);
                } else {
                    break;
                }
            }
            gl::GetTexImage(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH_COMPONENT,
                gl::FLOAT,
                std::ptr::null_mut()
                // 0,
                // 0,
                // self.width as i32,
                // self.height as i32,
                // gl::DEPTH_COMPONENT,
                // gl::FLOAT,
                // std::ptr::null_mut(),
            );
            loop {
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    error!("{}", error);
                } else {
                    break;
                }
            }
            self.pixel_buffers[(self.current + 3) % 4].unbind();
        }
    }
}

impl ZFinder for GLZFinder {
    fn get_z_at(&self, buffer_coordinate: BufferCoordinate) -> f32 {
        // return 0.0;
        if buffer_coordinate.y < 0 || buffer_coordinate.y >= self.height as i32 ||  buffer_coordinate.x < 0 || buffer_coordinate.x >= self.width as i32 {
            return 0.0;
        }
        let index = (buffer_coordinate.y * self.width as i32) + buffer_coordinate.x;
        let index = if index < 0 { return 0.0; } else { index as usize };
        let start = Instant::now();
        unsafe {
            self.pixel_buffers[self.current].bind();
            let out = self.pixel_buffers[self.current].read();
            self.pixel_buffers[self.current].unmap();
            self.pixel_buffers[self.current].unbind();
            if let Some(out) = out {
                let elapsed = start.elapsed().as_micros();
                if elapsed > 0 {
                    debug!("picking took {}ms got {}", elapsed, out[index]);
                }
                2.0 * out[index] - 1.0
            } else {
                0.0
            }
        }
    }
}
