use commons::image;
use image::{DynamicImage, GenericImageView};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;

pub struct Texture {
    id: gl::types::GLuint,
    width: u32,
    height: u32,
}

impl Texture {
    pub fn new(image: DynamicImage) -> Texture {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            let mut out = Texture {
                id,
                width: 0,
                height: 0,
            };
            out.load(image);
            out
        }
    }

    pub fn id(&self) -> &gl::types::GLuint {
        &self.id
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn bind(&self, slot: u32) {
        gl::ActiveTexture(gl::TEXTURE0 + slot);
        gl::BindTexture(gl::TEXTURE_2D, self.id);
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn unbind(&self, slot: u32) {
        gl::ActiveTexture(gl::TEXTURE0 + slot);
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    fn load(&mut self, image: DynamicImage) {
        let dimensions = image.dimensions();
        self.width = dimensions.0;
        self.height = dimensions.1;
        let image = image.to_rgba8().into_raw();
        let image_ptr: *const c_void = image.as_ptr() as *const c_void;

        unsafe {
            self.bind(0);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                self.width as i32,
                self.height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                image_ptr,
            );
            self.unbind(0);
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}

#[derive(Default)]
pub struct TextureLibrary {
    textures: HashMap<String, Arc<Texture>>,
}

impl TextureLibrary {
    pub fn get_texture(&mut self, file: &str) -> Arc<Texture> {
        self.textures
            .entry(file.to_string())
            .or_insert_with(|| Self::load_texture(file))
            .clone()
    }

    fn load_texture(file: &str) -> Arc<Texture> {
        let texture = Texture::new(image::open(file).unwrap());
        Arc::new(texture)
    }
}
