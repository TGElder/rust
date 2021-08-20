use super::*;

trait WorldValidator {
    fn validate(&self, world: &World) -> bool;
    fn invalid_message(&self) -> String;
}

struct RatioAboveSeaLevel {
    min_ratio: f32,
}

impl RatioAboveSeaLevel {
    pub fn new(min_ratio: f32) -> RatioAboveSeaLevel {
        RatioAboveSeaLevel { min_ratio }
    }
}

impl WorldValidator for RatioAboveSeaLevel {
    fn validate(&self, world: &World) -> bool {
        let width = world.width();
        let height = world.height();

        let mut sea = 0;
        for x in 0..width {
            for y in 0..height {
                if world.is_sea(&v2(x, y)) {
                    sea += 1;
                }
            }
        }
        let ratio_below_sea = sea as f32 / (width * height) as f32;
        let ratio_above_sea = 1.0 - ratio_below_sea;
        ratio_above_sea >= self.min_ratio
    }

    fn invalid_message(&self) -> String {
        format!("Less than {} of world is above sea level", self.min_ratio)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldValidationParams {
    min_ratio_above_sea_level: f32,
}

impl Default for WorldValidationParams {
    fn default() -> WorldValidationParams {
        WorldValidationParams {
            min_ratio_above_sea_level: 0.4,
        }
    }
}

pub fn world_is_valid(params: &WorldValidationParams, world: &World) -> bool {
    for validator in &[RatioAboveSeaLevel::new(params.min_ratio_above_sea_level)] {
        if !validator.validate(world) {
            println!("World is invalid: {}", validator.invalid_message());
            return false;
        }
    }
    true
}
