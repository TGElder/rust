use std::collections::HashMap;
use std::iter::once;

use commons::rectangle::Rectangle;
use isometric::coords::WorldCoord;
use isometric::drawing::{
    create_billboards, update_billboard_texture, update_billboards_vertices, Billboard,
};
use isometric::Command;

use crate::artists::avatar_artist::artist_avatar::ArtistAvatar;
use crate::artists::sprite_sheet::SpriteSheet;
use crate::avatar::AvatarLoad;

const LOAD_DRAWING: &str = "loads";

pub struct LoadArtist {
    params: LoadArtistParams,
    texture_coords: HashMap<String, Rectangle<f32>>,
}

pub struct LoadArtistParams {
    load_size: f32,
    load_height: f32,
    sprite_sheet_image: String,
    sprite_sheet_json: String,
}

impl Default for LoadArtistParams {
    fn default() -> Self {
        LoadArtistParams {
            load_size: 0.15,
            load_height: 0.3,
            sprite_sheet_image: "resources/textures/sprite_sheets/resources.png".to_string(),
            sprite_sheet_json: "resources/textures/sprite_sheets/resources.json".to_string(),
        }
    }
}

impl LoadArtist {
    pub fn new(params: LoadArtistParams) -> LoadArtist {
        LoadArtist {
            texture_coords: SpriteSheet::load(&params.sprite_sheet_json).texture_coords(),
            params,
        }
    }

    pub fn init(&self, max_avatars: usize) -> impl Iterator<Item = Command> {
        once(create_billboards(LOAD_DRAWING.to_string(), max_avatars)).chain(once(
            update_billboard_texture(LOAD_DRAWING.to_string(), &self.params.sprite_sheet_image),
        ))
    }

    pub fn draw_loads(&self, avatars: &[ArtistAvatar]) -> Command {
        let to_draw = self.get_what_to_draw(avatars);
        let billboards = to_draw
            .iter()
            .map(|(world_coord, texture_coords)| Billboard {
                world_coord,
                width: &self.params.load_size,
                height: &self.params.load_size,
                texture_coords,
            })
            .collect();
        update_billboards_vertices(LOAD_DRAWING.to_string(), billboards)
    }

    fn get_what_to_draw<'a>(
        &'a self,
        avatars: &[ArtistAvatar],
    ) -> Vec<(WorldCoord, &'a Rectangle<f32>)> {
        avatars
            .iter()
            .flat_map(|avatar| {
                self.get_texture_coords(avatar).map(|coords| {
                    let WorldCoord { x, y, z } = avatar.world_coord;
                    (WorldCoord::new(x, y, z + self.params.load_height), coords)
                })
            })
            .collect()
    }

    fn get_texture_coords(&self, avatar: &ArtistAvatar) -> Option<&Rectangle<f32>> {
        match avatar.progress.load() {
            AvatarLoad::Resource(resource) => self.texture_coords.get(resource.name()),
            _ => None,
        }
    }
}
