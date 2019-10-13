use commons::scale::*;
use commons::*;

pub struct TemperatureMapper {
    pub y_to_latitude: Scale<f64>,
    pub z_to_elevation: Scale<f64>,
    pub latitude_to_temperature: Scale<f64>,
    pub elevation_to_temperature: Scale<f64>,
    pub sea_level: f64,
}

impl TemperatureMapper {
    pub fn earthlike(
        y_to_latitude: Scale<f64>,
        z_to_elevation: Scale<f64>,
        sea_level: f64,
    ) -> TemperatureMapper {
        TemperatureMapper {
            y_to_latitude,
            z_to_elevation,
            latitude_to_temperature: Self::earthlike_latitude_to_temperature(),
            elevation_to_temperature: Self::earthlike_elevation_to_temperature(),
            sea_level,
        }
    }

    pub fn compute_temperature_map(&self, elevations: &M<f64>) -> M<f64> {
        let (width, height) = elevations.shape();
        M::from_fn(width, height, |x, y| {
            self.compute_temperature_at(x, y, elevations[(x, y)])
        })
    }

    fn compute_temperature_at(&self, _: usize, y: usize, z: f64) -> f64 {
        let latitude = self.y_to_latitude.scale(y as f64);
        let base_temperature = self.latitude_to_temperature.scale(latitude.abs());
        let elevation = self.z_to_elevation.scale(z.max(self.sea_level));
        let elevation_temperature = self.elevation_to_temperature.scale(elevation);
        base_temperature + elevation_temperature
    }

    fn earthlike_elevation_to_temperature() -> Scale<f64> {
        Scale::new((0.0, 100_000.0), (0.0, -600.0))
    }

    fn earthlike_latitude_to_temperature() -> Scale<f64> {
        Scale::new((0.0, 90.0), (35.0, -35.0))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_compute_temperature_at() {
        let mapper = TemperatureMapper {
            y_to_latitude: Scale::new((0.0, 100.0), (-55.0, -5.0)),
            z_to_elevation: Scale::new((0.0, 32.0), (0.0, 10000.0)),
            latitude_to_temperature: Scale::new((0.0, 90.0), (40.0, 0.0)),
            elevation_to_temperature: Scale::new((0.0, 3000.0), (0.0, -30.0)),
            sea_level: 0.0,
        };

        let actual = mapper.compute_temperature_at(11, 21, 1.0272);
        let expected = 17.012;

        assert!((actual - expected).abs() < 0.001);
    }

    #[test]
    fn test_compute_temperature_under_sea_level() {
        let mapper = TemperatureMapper {
            y_to_latitude: Scale::new((0.0, 100.0), (-55.0, -5.0)),
            z_to_elevation: Scale::new((0.0, 32.0), (0.0, 10000.0)),
            latitude_to_temperature: Scale::new((0.0, 90.0), (40.0, 0.0)),
            elevation_to_temperature: Scale::new((0.0, 3000.0), (0.0, -30.0)),
            sea_level: 1.0,
        };

        assert!(mapper
            .compute_temperature_at(11, 21, 0.0)
            .almost(mapper.compute_temperature_at(11, 21, 1.0)));
    }
}
