use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct TownArtistParameters {
    pub house_width: f32,
    pub house_roof_height: f32,
    pub house_height_log_base: f64,
}

impl Default for TownArtistParameters {
    fn default() -> TownArtistParameters {
        TownArtistParameters {
            house_width: 0.25,
            house_roof_height: 0.5,
            house_height_log_base: 10.0,
        }
    }
}
