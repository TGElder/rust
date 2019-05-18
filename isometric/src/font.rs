use commons::{v2, V2};
use graphics::texture::Texture;
use std::fs::File;
use std::io::Read;

#[derive(Clone, Copy)]
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
    fn from_line(line: &str) -> Glyph {
        let columns: Vec<&str> = line.split(",").collect();
        let id: usize = columns[0].parse().unwrap();
        Glyph {
            character: id as u8 as char,
            x: columns[1].parse().unwrap(),
            y: columns[2].parse().unwrap(),
            width: columns[3].parse().unwrap(),
            height: columns[4].parse().unwrap(),
            xoffset: columns[5].parse().unwrap(),
            yoffset: columns[6].parse().unwrap(),
            xadvance: columns[7].parse().unwrap(),
        }
    }

    pub fn from_csv(file_name: &str) -> [Option<Glyph>; 256] {
        let mut file = File::open(file_name).expect(&format!("Font file {} not found", file_name));
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect(&format!("Failed to read font file {}", file_name));

        let mut glyphs = [None; 256];

        for line in contents.split("\n") {
            let glyph = Glyph::from_line(line);
            glyphs[glyph.character as usize] = Some(glyph);
        }

        glyphs
    }
}

pub struct Font {
    glyphs: [Option<Glyph>; 256],
    texture: Texture,
}

impl Font {
    pub fn from_csv_and_texture(csv_file_name: &str, texture: Texture) -> Font {
        Font {
            glyphs: Glyph::from_csv(csv_file_name),
            texture,
        }
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    fn get_glyph(&self, character: char) -> Glyph {
        if character as usize > 255 {
            panic!("Rendering of character [{}] not supported - only first 256 characters are supported.", character);
        }

        self.glyphs[character as usize]
            .or(self.glyphs['?' as usize])
            .expect(&format!(
                "Rendering of character [{}] not supported in this font",
                character
            ))
    }

    pub fn get_dimensions(&self, character: char) -> (i32, i32) {
        let glyph = self.get_glyph(character);
        (glyph.width, glyph.height)
    }

    pub fn get_texture_coords(&self, character: char) -> (V2<f32>, V2<f32>) {
        let glyph = self.get_glyph(character);
        (
            self.texture.get_texture_coords(v2(glyph.x, glyph.y)),
            self.texture
                .get_texture_coords(v2(glyph.x + glyph.width, glyph.y + glyph.height)),
        )
    }

    pub fn get_advance(&self, character: char) -> i32 {
        self.get_glyph(character).xadvance
    }

    pub fn get_width(&self, text: &str) -> i32 {
        text.chars().map(|c| self.get_glyph(c).xadvance).sum()
    }

    pub fn get_offset(&self, character: char) -> (i32, i32) {
        let glyph = self.get_glyph(character);
        (glyph.xoffset, glyph.yoffset)
    }
}
