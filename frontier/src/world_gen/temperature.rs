use super::*;
use crate::world::*;
use commons::scale::*;
use commons::*;
use pioneer::sunshine::*;
use pioneer::temperature::*;
use std::default::Default;
use std::f64::consts::PI;

const MAX_ELEVATION_METRES: f64 = 4000.0;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct TemperatureParams {
    pub sunshine_adjustment_range: (f64, f64),
}

impl Default for TemperatureParams {
    fn default() -> TemperatureParams {
        TemperatureParams {
            sunshine_adjustment_range: (-15.0, 0.0),
        }
    }
}

fn y_to_latitude(world: &World, latitude_range: &(f64, f64)) -> Scale<f64> {
    let height = world.height() as f64;
    Scale::new((0.0, height), *latitude_range)
}

fn z_to_elevation(world: &World) -> Scale<f64> {
    let sea_level = f64::from(world.sea_level());
    let max_height = f64::from(world.max_height());
    Scale::new((sea_level, max_height), (0.0, MAX_ELEVATION_METRES))
}

pub fn compute_temperatures(world: &World, params: &WorldGenParameters) -> M<f64> {
    let elevations = extract_matrix(world, &|cell| f64::from(cell.elevation));
    let y_to_latitude = y_to_latitude(world, &params.latitude_range);
    let z_to_elevation = z_to_elevation(world);
    let sunshine_temperatures = compute_sunshine_temperatures(&elevations, &y_to_latitude, params);
    let mapper =
        TemperatureMapper::earthlike(y_to_latitude, z_to_elevation, f64::from(world.sea_level()));
    let base_temperatures = mapper.compute_temperature_map(&elevations);
    base_temperatures + sunshine_temperatures
}

fn compute_sunshine_temperatures(
    elevations: &M<f64>,
    y_to_latitude: &Scale<f64>,
    params: &WorldGenParameters,
) -> M<f64> {
    let sunshine = sunshine(elevations, y_to_latitude);
    let range = params.temperature.sunshine_adjustment_range;
    let sunshine_scale = Scale::new((0.0, PI), (range.1, range.0));
    sunshine.map(|v| sunshine_scale.scale(v))
}

pub fn load_temperatures(world: &mut World, temperatures: &M<f64>) {
    for x in 0..temperatures.width() {
        for y in 0..temperatures.height() {
            let position = v2(x, y);
            world.mut_cell_unsafe(&position).climate.temperature =
                *temperatures.get_cell_unsafe(&position) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_y_to_latitude() {
        let world = World::new(M::from_element(1, 10, 0.0), 0.0);
        assert_eq!(
            y_to_latitude(&world, &(10.0, 55.0)),
            Scale::new((0.0, 10.0), (10.0, 55.0))
        );
    }

    #[rustfmt::skip]
    #[test]
    fn test_z_to_elevation() {
        let world = World::new(M::from_vec(3, 3, 
        vec![
            0.0, 1.0, 2.0,
            1.0, 2.0, 3.0,
            1.0, 0.0, 4.0
        ]).transpose(), 1.0);
        assert_eq!(
            z_to_elevation(&world),
            Scale::new((1.0, 4.0), (0.0, MAX_ELEVATION_METRES))
        );
    }
}
