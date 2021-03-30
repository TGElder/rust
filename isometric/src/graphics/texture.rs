use commons::image;
use commons::log::debug;
use image::{DynamicImage, GenericImageView};
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::Path;
use std::sync::Arc;

pub struct Texture {
    id: gl::types::GLuint,
    width: u32,
    height: u32,
}

impl Texture {
    pub fn new(images: Vec<DynamicImage>) -> Texture {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            let mut out = Texture {
                id,
                width: 0,
                height: 0,
            };
            out.load_images(images);
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

    fn load_images(&mut self, mut images: Vec<DynamicImage>) {
        images.sort_by_key(|image| image.dimensions().0);
        images.reverse();
        unsafe {
            self.bind(0);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::NEAREST_MIPMAP_NEAREST as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::NEAREST as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAX_LEVEL,
                images.len() as i32 - 1,
            );
            for (level, image) in images.into_iter().enumerate() {
                self.load_image(image, level as i32);
            }
            self.unbind(0);
        }
    }

    unsafe fn load_image(&mut self, image: DynamicImage, level: i32) {
        let dimensions = image.dimensions();
        self.width = dimensions.0;
        self.height = dimensions.1;
        let image = image.to_rgba().into_raw();
        let image_ptr: *const c_void = image.as_ptr() as *const c_void;
        debug!("Loading {}x{} image", dimensions.0, dimensions.1);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            level,
            gl::RGBA as i32,
            dimensions.0 as i32,
            dimensions.1 as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            image_ptr,
        );
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
        let path = Path::new(file);
        if std::path::Path::is_dir(&path) {
            let images = path
                .read_dir()
                .unwrap()
                .map(|entry| image::open(entry.unwrap().path().to_str().unwrap()).unwrap())
                .collect();
            let texture = Texture::new(images);
            Arc::new(texture)
        } else {
            let texture = Texture::new(vec![image::open(file).unwrap()]);
            Arc::new(texture)
        }
    }
}
