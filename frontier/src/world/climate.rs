use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Climate {
    pub temperature: f32,
    pub rainfall: f32,
    pub vegetation_elevation: f32,
    pub river_water: f32,
}

impl Climate {
    pub fn groundwater(&self) -> f32 {
        self.rainfall.max(self.river_water)
    }
}
