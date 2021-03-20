use crate::graphics::program::Program;
use crate::graphics::vertex_objects::MultiVBO;
use crate::graphics::DrawingType;

pub struct FrameBuffer {
    id: gl::types::GLuint,
    color_buffer: FrameBufferTexture,
    depth_buffer: RenderBuffer,
    vbo: MultiVBO,
}

impl FrameBuffer {
    pub fn new(width: i32, height: i32) -> FrameBuffer {
        let mut id: gl::types::GLuint = 0;
        let color_buffer = FrameBufferTexture::new(width, height, gl::RGBA, gl::UNSIGNED_BYTE);
        let depth_buffer = RenderBuffer::new(width, height);
        let mut vbo = MultiVBO::new(DrawingType::FullScreenQuad, 1, 24);
        vbo.load(
            0,
            vec![
                -1.0, 1.0, 0.0, 1.0, -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, -1.0, 1.0, 0.0,
                1.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
            ],
        );
        unsafe {
            gl::GenFramebuffers(1, &mut id);
        }
        let out = FrameBuffer {
            id,
            color_buffer,
            depth_buffer,
            vbo,
        };
        out.bind();
        unsafe {
            out.attach_color_buffer(&out.color_buffer);
            out.attach_depth_buffer(&out.depth_buffer);
            out.check_complete();
        }
        out.unbind();
        out
    }

    pub fn begin_drawing(&self) {
        self.bind();
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::Enable(gl::DEPTH_TEST);
        }
    }

    pub fn copy_to_back_buffer(&self, program: &Program) {
        self.unbind();
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::Disable(gl::DEPTH_TEST);
            program.set_used();
            program.link_texture_slot_to_variable(0, "screenTexture");
            self.color_buffer.bind();
            self.vbo.draw();
            self.color_buffer.unbind();
        }
    }

    fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    unsafe fn attach_color_buffer(&self, texture: &FrameBufferTexture) {
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            texture.id,
            0,
        );
    }

    unsafe fn attach_depth_buffer(&self, buffer: &RenderBuffer) {
        gl::FramebufferRenderbuffer(
            gl::FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::RENDERBUFFER,
            buffer.id,
        );
    }

    unsafe fn check_complete(&self) {
        let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
        if status != gl::FRAMEBUFFER_COMPLETE {
            panic!(
                "FBO was not successfully created, expected status {} but recevied {}",
                gl::FRAMEBUFFER_COMPLETE,
                status
            );
        }
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.id);
        }
    }
}

pub struct FrameBufferTexture {
    id: gl::types::GLuint,
}

impl FrameBufferTexture {
    pub fn new(
        width: i32,
        height: i32,
        format: gl::types::GLenum,
        type_: gl::types::GLenum,
    ) -> FrameBufferTexture {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            let mut out = FrameBufferTexture { id };
            out.init(width, height, format, type_);
            out
        }
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn bind(&self) {
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.id);
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn unbind(&self) {
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    fn init(
        &mut self,
        width: i32,
        height: i32,
        format: gl::types::GLenum,
        type_: gl::types::GLenum,
    ) {
        unsafe {
            self.bind();
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                format as i32,
                width,
                height,
                0,
                format,
                type_,
                std::ptr::null(),
            );
            self.unbind();
        }
    }
}

impl Drop for FrameBufferTexture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}

struct RenderBuffer {
    id: gl::types::GLuint,
}

impl RenderBuffer {
    pub fn new(width: i32, height: i32) -> RenderBuffer {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenRenderbuffers(1, &mut id);
        }
        let out = RenderBuffer { id };
        unsafe {
            out.bind();
            out.init(width, height);
            out.unbind();
        }
        out
    }

    unsafe fn bind(&self) {
        gl::BindRenderbuffer(gl::RENDERBUFFER, self.id);
    }

    unsafe fn unbind(&self) {
        gl::BindRenderbuffer(gl::RENDERBUFFER, 0);
    }

    unsafe fn init(&self, width: i32, height: i32) {
        gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH_COMPONENT, width, height);
    }
}

impl Drop for RenderBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteRenderbuffers(1, &self.id);
        }
    }
}
