use commons::image::{self, ImageError};
use commons::log::{debug, error};
use image::{DynamicImage, GenericImageView};
use std::collections::HashMap;
use std::ffi::c_void;
use std::io;
use std::path::Path;
use std::sync::Arc;

pub struct Texture {
    id: gl::types::GLuint,
}

impl Texture {
    pub fn new(images: Vec<DynamicImage>) -> Texture {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            let mut out = Texture { id };
            out.load_images(images);
            out
        }
    }

    pub fn id(&self) -> &gl::types::GLuint {
        &self.id
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
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
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
        debug!("Loading {}x{} image to level {}", image.width(), image.height(), level);
        let dimensions = image.dimensions();
        let image = image.to_rgba().into_raw();
        let image_ptr: *const c_void = image.as_ptr() as *const c_void;
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
    pub fn get_texture(&mut self, path: &str) -> Arc<Texture> {
        self.textures
            .entry(path.to_string())
            .or_insert_with(|| {
                Self::load_texture(path)
                    .unwrap_or_else(|err| panic!("Could not load texture from {}: {:?}", path, err))
            })
            .clone()
    }

    fn load_texture(path: &str) -> Result<Arc<Texture>, TextureLibraryError> {
        let path = Path::new(path);
        if path.is_dir() {
            Self::load_directory(path)
        } else {
            Self::load_file(path)
        }
    }

    fn load_directory(directory: &Path) -> Result<Arc<Texture>, TextureLibraryError> {
        let mut images = vec![];
        for file in directory.read_dir()? {
            images.push(image::open(
                file?.path().to_str().ok_or("Non-unicode path")?,
            )?);
        }
        let texture = Texture::new(images);
        Ok(Arc::new(texture))
    }

    fn load_file(path: &Path) -> Result<Arc<Texture>, TextureLibraryError> {
        let texture = Texture::new(vec![image::open(path)?]);
        Ok(Arc::new(texture))
    }
}

#[derive(Debug)]
pub enum TextureLibraryError {
    IO(io::Error),
    Str(&'static str),
    Image(ImageError),
}

impl From<io::Error> for TextureLibraryError {
    fn from(error: io::Error) -> Self {
        Self::IO(error)
    }
}

impl From<&'static str> for TextureLibraryError {
    fn from(error: &'static str) -> Self {
        Self::Str(error)
    }
}

impl From<ImageError> for TextureLibraryError {
    fn from(error: ImageError) -> Self {
        Self::Image(error)
    }
}
