use commons::{v3, V3};
use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct TownArtistParameters {
    pub house_width: f32,
    pub house_roof_height: f32,
    pub house_height_log_base: f64,
    pub light_direction: V3<f32>,
    pub label_float: f32,
}

impl Default for TownArtistParameters {
    fn default() -> TownArtistParameters {
        TownArtistParameters {
            house_width: 0.25,
            house_roof_height: 0.5,
            house_height_log_base: 10.0,
            light_direction: v3(0.0, 8.0, -1.0),
            label_float: 0.33,
        }
    }
}
