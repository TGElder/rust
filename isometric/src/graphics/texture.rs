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

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub unsafe fn bind(&self) {
        gl::BindTexture(gl::TEXTURE_2D, self.id);
    }

    pub unsafe fn unbind(&self) {
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    fn load(&mut self, image: DynamicImage) {
        let dimensions = image.dimensions();
        self.width = dimensions.0;
        self.height = dimensions.1;
        let image = image.to_rgba().into_raw();
        let image_ptr: *const c_void = image.as_ptr() as *const c_void;

        unsafe {
            self.bind();
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
            self.unbind();
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &mut self.id);
        }
    }
}

pub struct TextureLibrary {
    textures: HashMap<String, Arc<Texture>>,
}

impl TextureLibrary {
    pub fn new() -> TextureLibrary {
        TextureLibrary {
            textures: HashMap::new(),
        }
    }

    pub fn get_texture(&mut self, file: &String) -> Arc<Texture> {
        self.textures
            .entry(file.clone())
            .or_insert_with(|| Self::load_texture(file))
            .clone()
    }

    fn load_texture(file: &String) -> Arc<Texture> {
        let texture = Texture::new(image::open(file).unwrap());
        Arc::new(texture)
    }
}
