use super::*;
use crate::world::*;
use commons::grid::extract_matrix;
use commons::*;
use pioneer::rain::*;
use std::default::Default;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct RainfallGenParams {
    pub air_capacity_range: (f64, f64),
    // percentage of world travelled for 99% moisture within capacity to be lost
    pub under_capacity_99pc_distance: f64,
    // percentage of world travelled for 99% moisture over ca[acity to be lost
    pub over_capacity_99pc_distance: f64,
    // percentage of world travelled over water for evaporation to fill 99% of capacity
    pub evaporation_99pc_distance: f64,
    pub wind_probabilities: [f64; 8],
}

impl Default for RainfallGenParams {
    fn default() -> RainfallGenParams {
        RainfallGenParams {
            air_capacity_range: (1.0, 0.0),
            under_capacity_99pc_distance: 1.0,
            over_capacity_99pc_distance: 0.25,
            evaporation_99pc_distance: 1.0,
            wind_probabilities: [0.1, 0.13, 0.15, 0.17, 0.15, 0.13, 0.1, 0.08],
        }
    }
}

pub fn gen_rainfall(world: &World, params: &WorldGenParameters) -> M<f64> {
    let elevations = extract_matrix(world, &|cell| f64::from(cell.elevation));
    let capacities = rescale(elevations.clone(), params.rainfall.air_capacity_range);
    let mut computer = RainfallComputer {
        params: RainfallParams {
            winds: RainfallParams::equal_probability_winds(),
            under_capacity_rain_probability: calculate_probability(
                0.01,
                params.rainfall.under_capacity_99pc_distance,
                world.width(),
            ),
            over_capacity_rain_probability: calculate_probability(
                0.01,
                params.rainfall.over_capacity_99pc_distance,
                world.width(),
            ),
            evaporation_probability: calculate_probability(
                0.01,
                params.rainfall.evaporation_99pc_distance,
                world.width(),
            ),
        },
        elevations: &elevations,
        capacities,
        sea_level: f64::from(world.sea_level()),
    };
    computer
        .params
        .set_probabilities(params.rainfall.wind_probabilities);
    let rain = computer.compute();
    rescale_ignoring_sea(rain, world)
}

fn calculate_probability(
    percentage_left: f64,
    percentage_of_world_travelled: f64,
    world_width: usize,
) -> f64 {
    let exponent = 1.0 / (world_width as f64 * percentage_of_world_travelled);
    1.0 - percentage_left.powf(exponent)
}

pub fn load_rainfall(world: &mut World, rainfall: &M<f64>) {
    for x in 0..rainfall.width() {
        for y in 0..rainfall.height() {
            let position = v2(x, y);
            world.mut_cell_unsafe(&position).climate.rainfall =
                *rainfall.get_cell_unsafe(&position) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_probability() {
        assert!(calculate_probability(0.01, 1.0 / 16.0, 1024) - 0.0694 <= 0.0001)
    }
}
