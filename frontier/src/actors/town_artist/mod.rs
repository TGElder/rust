mod params;
mod town_houses;
mod town_labels;

pub use params::*;
pub use town_houses::*;
pub use town_labels::*;

use crate::settlement::Settlement;

fn get_house_height_with_roof(params: &TownArtistParameters, settlement: &Settlement) -> f32 {
    get_house_height_without_roof(params, settlement) + params.house_roof_height
}

fn get_house_height_without_roof(params: &TownArtistParameters, settlement: &Settlement) -> f32 {
    (settlement.current_population + 1.0).log(params.house_height_log_base) as f32
}
