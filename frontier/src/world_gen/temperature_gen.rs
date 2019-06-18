use crate::world::*;
use commons::scale::*;
use commons::*;
use isometric::cell_traits::*;
use pioneer::temperature::*;

const MIN_LATITUDE: f64 = 0.0;
const MAX_LATITUDE: f64 = 50.0;
const MAX_ELEVATION: f64 = 4000.0;

fn y_to_latitude(world: &World) -> Scale<f64> {
    let height = world.height() as f64;
    Scale::new((0.0, height), (MIN_LATITUDE, MAX_LATITUDE))
}

fn z_to_elevation(world: &World) -> Scale<f64> {
    let sea_level = world.sea_level() as f64;
    let max_height = world.max_height() as f64;
    Scale::new((sea_level, max_height), (0.0, MAX_ELEVATION))
}

pub fn setup_temperatures(world: &mut World) {
    let elevations = M::from_fn(world.width(), world.height(), |x, y| {
        world.get_cell(&v2(x, y)).unwrap().elevation() as f64
    });
    let y_to_latitude = y_to_latitude(world);
    let z_to_elevation = z_to_elevation(world);
    let temperature = TemperatureMapper::earthlike(y_to_latitude, z_to_elevation)
        .compute_temperature_map(&elevations);
    for x in 0..world.width() {
        for y in 0..world.height() {
            world.mut_cell_unsafe(&v2(x, y)).climate = Climate {
                temperature: temperature[(x, y)] as f32,
            }
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
            y_to_latitude(&world),
            Scale::new((0.0, 10.0), (MIN_LATITUDE, MAX_LATITUDE))
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
            Scale::new((1.0, 4.0), (0.0, MAX_ELEVATION))
        );
    }
}
