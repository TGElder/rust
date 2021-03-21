use std::slice;

pub struct PixelBuffer {
    id: gl::types::GLuint,
    width: usize,
    height: usize,
}

impl PixelBuffer {
    pub fn new(width: usize, height: usize) -> PixelBuffer {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut id);
        }
        let out = PixelBuffer { id, width, height };
        unsafe {
            out.bind();
            out.init();
            out.unbind();
        }
        out
    }

    pub unsafe fn init(&self) {
        
        gl::BufferData(gl::PIXEL_PACK_BUFFER, (self.width * self.height * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,  std::ptr::null_mut(), gl::STREAM_READ);
    }

    pub unsafe fn read(&self) -> Option<&[f32]> {
        let ptr = gl::MapBuffer(gl::PIXEL_PACK_BUFFER, gl::READ_ONLY);
        if ptr.is_null() {
            None
        } else {
            Some(slice::from_raw_parts(ptr as *mut f32, (self.width * self.height) as usize))
        }
    }

    pub unsafe fn unmap(&self) {
        gl::UnmapBuffer(gl::PIXEL_PACK_BUFFER);
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
