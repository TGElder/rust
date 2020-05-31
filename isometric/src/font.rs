use commons::rectangle::Rectangle;
use commons::{v2, V2};
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const EXTRACT_PATTERN: &'static str = "=\"?([^\\s\"]+)\"?\\s";
const GLYPH_PATTERN: &'static str = "char id=(\\d+)\\s*x=(\\d+)\\s*y=(\\d+)\\s*width=(\\d+)\\s*height=(\\d+)\\s*xoffset=(-?\\d+)\\s*yoffset=(-?\\d+)\\s*xadvance=(\\d+).*";
const KERNING_PATTERN: &'static str = "kerning first=(\\d+)\\s*second=(\\d+)\\s*amount=(-?\\d+).*";
const UNKNOWN_CHARACTER: char = '?';

pub struct Font {
    glyphs: HashMap<char, Glyph>,
    kernings: HashMap<(char, char), i32>,
    texture: String,
    texture_width: f32,
    texture_height: f32,
    base: f32,
}

impl Font {
    pub fn from_file(file_name: &str) -> Font {
        let mut file =
            File::open(file_name).unwrap_or_else(|_| panic!("Font file {} not found", file_name));
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .unwrap_or_else(|_| panic!("Failed to read font file {}", file_name));
        let directory = Path::new(file_name).parent();
        Font::from_text(&contents, directory)
    }

    fn from_text(text: &str, directory: Option<&Path>) -> Font {
        Font {
            glyphs: Glyph::from_text(text),
            kernings: Font::kernings_from_text(text),
            texture: with_directory(extract_value_unsafe("file", text), directory),
            texture_width: extract_value_unsafe("scaleW", text).parse().unwrap(),
            texture_height: extract_value_unsafe("scaleH", text).parse().unwrap(),
            base: extract_value_unsafe("base", text).parse().unwrap(),
        }
    }

    fn kerning_from_captures(captures: &Captures) -> ((char, char), i32) {
        let first: u8 = captures[1].parse().unwrap();
        let second: u8 = captures[2].parse().unwrap();
        let amount = captures[3].parse().unwrap();
        ((first as char, second as char), amount)
    }

    fn kernings_from_text(text: &str) -> HashMap<(char, char), i32> {
        let kerning_regex = Regex::new(KERNING_PATTERN)
            .expect(&format!("Cannot find {:?} in font file", KERNING_PATTERN));

        kerning_regex
            .captures_iter(&text)
            .map(|captures| Font::kerning_from_captures(&captures))
            .collect()
    }

    pub fn texture(&self) -> &str {
        &self.texture
    }

    pub fn base(&self) -> f32 {
        self.base
    }

    fn get_glyph(&self, character: char) -> &Glyph {
        self.glyphs
            .get(&character)
            .or(self.glyphs.get(&UNKNOWN_CHARACTER))
            .unwrap_or_else(|| {
                panic!(
                    "Could not render character [{}] or fallback [{}]",
                    character, UNKNOWN_CHARACTER
                )
            })
    }

    pub fn get_kerning(&self, first: char, second: char) -> i32 {
        *self.kernings.get(&(first, second)).unwrap_or(&0)
    }

    pub fn get_dimensions(&self, character: char) -> V2<i32> {
        let glyph = self.get_glyph(character);
        v2(glyph.width, glyph.height)
    }

    pub fn get_texture_coords(&self, character: char) -> Rectangle<f32> {
        let glyph = self.get_glyph(character);
        Rectangle {
            from: self.get_texture_coord(v2(glyph.x, glyph.y)),
            to: self.get_texture_coord(v2(glyph.x + glyph.width, glyph.y + glyph.height)),
        }
    }

    fn get_texture_coord(&self, pixel_position: V2<i32>) -> V2<f32> {
        v2(
            pixel_position.x as f32 / self.texture_width,
            pixel_position.y as f32 / self.texture_height,
        )
    }

    pub fn get_advance(&self, character: char) -> i32 {
        self.get_glyph(character).xadvance
    }

    pub fn get_width(&self, text: &str) -> i32 {
        text.chars().map(|c| self.get_glyph(c).xadvance).sum()
    }

    pub fn get_offset(&self, character: char) -> V2<i32> {
        let glyph = self.get_glyph(character);
        v2(glyph.xoffset, glyph.yoffset)
    }
}

fn extract_value_unsafe<'a>(key: &str, text: &'a str) -> &'a str {
    let pattern = format!("{}{}", key, EXTRACT_PATTERN);
    let regex = Regex::new(&pattern).unwrap();
    regex
        .captures(text)
        .expect(&format!("Cannot find {:?} in font file", pattern))
        .get(1)
        .unwrap()
        .as_str()
}

fn with_directory(file_name: &str, directory: Option<&Path>) -> String {
    directory
        .map(|path| path.join(file_name).to_string_lossy().to_string())
        .unwrap_or(file_name.to_string())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Glyph {
    character: char,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    xoffset: i32,
    yoffset: i32,
    xadvance: i32,
}

impl Glyph {
    pub fn from_text(text: &str) -> HashMap<char, Glyph> {
        let glyph_regex = Regex::new(GLYPH_PATTERN)
            .expect(&format!("Cannot find {:?} in font file", GLYPH_PATTERN));

        glyph_regex
            .captures_iter(&text)
            .map(|captures| Glyph::from_captures(&captures))
            .collect()
    }

    fn from_captures(captures: &Captures) -> (char, Glyph) {
        let character: u8 = captures[1].parse().unwrap();
        (
            character as char,
            Glyph {
                character: character as char,
                x: captures[2].parse().unwrap(),
                y: captures[3].parse().unwrap(),
                width: captures[4].parse().unwrap(),
                height: captures[5].parse().unwrap(),
                xoffset: captures[6].parse().unwrap(),
                yoffset: captures[7].parse().unwrap(),
                xadvance: captures[8].parse().unwrap(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_font() -> Font {
        Font::from_file("resources/test_font.fnt")
    }

    #[test]
    fn test_metadata() {
        let font = test_font();
        assert_eq!(font.texture, "resources/test_font_0.png".to_string());
        assert_eq!(font.texture_width, 256.0);
        assert_eq!(font.texture_height, 256.0);
        assert_eq!(font.base, 16.0);
    }

    #[test]
    fn test_glyphs() {
        let glyphs = test_font().glyphs;

        assert_eq!(glyphs.len(), 192);

        assert_eq!(
            *glyphs.get(&'3').unwrap(),
            Glyph {
                character: '3',
                x: 227,
                y: 149,
                width: 10,
                height: 13,
                xoffset: -1,
                yoffset: 4,
                xadvance: 8,
            }
        );
    }

    #[test]
    fn test_kernings() {
        let kernings = test_font().kernings;

        assert_eq!(kernings.len(), 475);

        assert_eq!(*kernings.get(&('A', 'G')).unwrap(), -1);
    }
}
