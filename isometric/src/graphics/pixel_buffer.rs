use std::slice;

pub struct PixelBuffer {
    id: gl::types::GLuint,
}

impl PixelBuffer {
    pub fn new() -> PixelBuffer {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut id);
        }
        let out = PixelBuffer { id };
        unsafe {
            out.bind();
            out.unbind();
        }
        out
    }

    pub unsafe fn read(&self) -> Option<&[f32]> {
        let ptr = gl::MapBuffer(gl::PIXEL_PACK_BUFFER, gl::READ_ONLY);
        if ptr.is_null() {
            None
        } else {
            Some(slice::from_raw_parts(ptr as *mut f32, 1))
        }
    }

    pub unsafe fn bind(&self) {
        gl::BindBuffer(gl::PIXEL_PACK_BUFFER, self.id);
    }

    pub unsafe fn unbind(&self) {
        gl::BindBuffer(gl::PIXEL_PACK_BUFFER, 0);
    }
}

impl Drop for PixelBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.id);
        }
    }
}
