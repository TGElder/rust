use commons::*;

#[derive(Clone)]
pub struct Wind {
    direction: V2<i32>,
    probability: f64,
}

#[derive(Debug, PartialEq)]
pub struct Rain {
    position: V2<usize>,
    amount: f64,
}

#[derive(Debug, PartialEq)]
pub struct Cloud {
    position: V2<usize>,
    direction: V2<i32>,
    capacity: f64,
    volume: f64,
    rain: Vec<Rain>,
}

impl Cloud {
    fn blow<T>(&mut self, grid: &Grid<T>) -> bool {
        if let Some(next) = grid.offset(&self.position, &self.direction) {
            self.position = next;
            return true;
        } else {
            return false;
        }
    }

    fn evaporate(&mut self, evaporation_probability: f64) {
        self.volume =
            self.volume + ((self.capacity - self.volume).max(0.0) * evaporation_probability);
    }

    fn under_capacity(&self) -> f64 {
        self.capacity.min(self.volume)
    }

    fn over_capacity(&self) -> f64 {
        self.volume - self.under_capacity()
    }

    fn rain(&mut self, amount: f64) {
        self.volume -= amount;
        self.rain.push(Rain {
            position: self.position,
            amount,
        });
    }

    fn rain_under_capacity(&mut self, probability: f64) {
        let amount = self.under_capacity() * probability;
        self.rain(amount);
    }

    fn rain_over_capacity(&mut self, probability: f64) {
        let amount = self.over_capacity() * probability;
        self.rain(amount);
    }
}

pub struct RainfallParams {
    pub winds: [Wind; 8],
    pub under_capacity_rain_probability: f64,
    pub over_capacity_rain_probability: f64,
    pub evaporation_probability: f64,
}

impl RainfallParams {
    pub fn equal_probability_winds() -> [Wind; 8] {
        [
            Wind {
                direction: v2(-1, -1),
                probability: 0.125,
            },
            Wind {
                direction: v2(0, -1),
                probability: 0.125,
            },
            Wind {
                direction: v2(1, -1),
                probability: 0.125,
            },
            Wind {
                direction: v2(1, 0),
                probability: 0.125,
            },
            Wind {
                direction: v2(1, 1),
                probability: 0.125,
            },
            Wind {
                direction: v2(0, 1),
                probability: 0.125,
            },
            Wind {
                direction: v2(-1, 1),
                probability: 0.125,
            },
            Wind {
                direction: v2(-1, 0),
                probability: 0.125,
            },
        ]
    }

    pub fn set_probabilities(&mut self, probabilities: [f64; 8]) {
        for i in 0..8 {
            self.winds[i].probability = probabilities[i]
        }
    }
}

pub struct RainfallComputer<'a> {
    pub params: RainfallParams,
    pub elevations: &'a M<f64>,
    pub capacities: M<f64>,
    pub sea_level: f64,
}

impl<'a> RainfallComputer<'a> {
    fn empty(&self) -> M<f64> {
        let (width, height) = self.elevations.shape();
        M::zeros(width, height)
    }

    pub fn compute(&self) -> M<f64> {
        let mut out = self.empty();
        for wind in self.params.winds.iter() {
            out += self.compute_wind(&wind);
        }
        rescale(out, (0.0, 1.0))
    }

    fn compute_wind(&self, wind: &Wind) -> M<f64> {
        let mut out = self.empty();
        self.elevations
            .edge_cells()
            .iter()
            .flat_map(|position| self.compute_cloud(*position, wind.direction))
            .for_each(|rain| {
                *out.mut_cell_unsafe(&rain.position) += rain.amount * wind.probability
            });
        out
    }

    fn valid_cloud(&self, start_at: &V2<usize>, direction: &V2<i32>) -> bool {
        if let Some(next) = self.elevations.offset(start_at, direction) {
            if self.elevations.is_corner_cell(start_at) {
                return true;
            } else if !self.elevations.is_edge_cell(start_at)
                || !self.elevations.is_edge_cell(&next)
            {
                return true;
            }
        }
        return false;
    }

    fn compute_cloud(&self, start_at: V2<usize>, direction: V2<i32>) -> Vec<Rain> {
        if !self.valid_cloud(&start_at, &direction) {
            return vec![];
        }
        let capacity = *self.capacities.get_cell_unsafe(&start_at);
        let mut cloud = Cloud {
            position: start_at,
            direction,
            capacity,
            volume: capacity,
            rain: vec![],
        };
        loop {
            self.compute_at(&mut cloud);
            if !cloud.blow(self.elevations) {
                return cloud.rain;
            }
        }
    }

    fn compute_at(&self, cloud: &mut Cloud) {
        cloud.capacity = *self.capacities.get_cell_unsafe(&cloud.position);
        if *self.elevations.get_cell_unsafe(&cloud.position) <= self.sea_level {
            cloud.evaporate(self.params.evaporation_probability);
        } else {
            cloud.rain_over_capacity(self.params.over_capacity_rain_probability);
            cloud.rain_under_capacity(self.params.under_capacity_rain_probability);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn params() -> RainfallParams {
        RainfallParams {
            winds: RainfallParams::equal_probability_winds(),
            under_capacity_rain_probability: 0.1,
            over_capacity_rain_probability: 0.5,
            evaporation_probability: 0.1,
        }
    }

    fn cloud() -> Cloud {
        Cloud {
            position: v2(0, 0),
            direction: v2(1, 0),
            capacity: 1.0,
            volume: 0.0,
            rain: vec![],
        }
    }

    #[test]
    fn blow() {
        let mut cloud = cloud();
        cloud.position = v2(1, 0);
        cloud.direction = v2(1, 0);
        assert_eq!(cloud.blow(&M::<u8>::zeros(8, 1)), true);
        assert_eq!(cloud.position, v2(2, 0));
    }

    #[test]
    fn blow_should_return_false_at_end() {
        let mut cloud = cloud();
        cloud.position = v2(7, 1);
        assert_eq!(cloud.blow(&M::<u8>::zeros(8, 1)), false);
    }

    #[test]
    fn evaporate_under_capacity() {
        let mut cloud = cloud();
        cloud.volume = 0.5;
        cloud.capacity = 1.0;
        cloud.evaporate(0.1);
        assert_eq!(cloud.volume, 0.5 + (0.1 * 0.5));
    }

    #[test]
    fn evaporate_over_capacity() {
        let mut cloud = cloud();
        cloud.volume = 1.1;
        cloud.capacity = 1.0;
        cloud.evaporate(0.1);
        assert_eq!(cloud.volume, 1.1);
    }

    #[test]
    fn under_capacity_under_capacity() {
        let mut cloud = cloud();
        cloud.volume = 0.25;
        cloud.capacity = 1.0;
        assert_eq!(cloud.under_capacity(), 0.25);
    }

    #[test]
    fn under_capacity_over_capacity() {
        let mut cloud = cloud();
        cloud.volume = 1.25;
        cloud.capacity = 1.0;
        assert_eq!(cloud.under_capacity(), 1.0);
    }

    #[test]
    fn over_capacity_under_capacity() {
        let mut cloud = cloud();
        cloud.volume = 0.25;
        cloud.capacity = 1.0;
        assert_eq!(cloud.over_capacity(), 0.0);
    }

    #[test]
    fn over_capacity_over_capacity() {
        let mut cloud = cloud();
        cloud.volume = 1.25;
        cloud.capacity = 1.0;
        assert_eq!(cloud.over_capacity(), 0.25);
    }

    #[test]
    fn rain() {
        let mut cloud = cloud();
        cloud.position = v2(0, 0);
        cloud.rain = vec![];
        cloud.rain(1.0);
        cloud.position = v2(1, 0);
        cloud.rain(0.1);
        assert_eq!(
            cloud.rain,
            vec![
                Rain {
                    position: v2(0, 0),
                    amount: 1.0
                },
                Rain {
                    position: v2(1, 0),
                    amount: 0.1
                }
            ]
        );
    }

    #[test]
    fn rain_under_capacity() {
        let mut cloud = cloud();
        cloud.position = v2(0, 0);
        cloud.rain = vec![];
        cloud.volume = 0.25;
        cloud.capacity = 1.0;
        cloud.rain_under_capacity(0.5);
        assert_eq!(
            cloud.rain,
            vec![Rain {
                position: v2(0, 0),
                amount: 0.125
            }]
        );
    }

    #[test]
    fn rain_over_capacity() {
        let mut cloud = cloud();
        cloud.position = v2(0, 0);
        cloud.rain = vec![];
        cloud.volume = 1.25;
        cloud.capacity = 1.0;
        cloud.rain_over_capacity(0.5);
        assert_eq!(
            cloud.rain,
            vec![Rain {
                position: v2(0, 0),
                amount: 0.125
            }]
        );
    }

    #[test]
    fn compute_at_under_sea_level() {
        let elevations = M::from_vec(8, 1, vec![0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let computer = RainfallComputer {
            params: params(),
            elevations: &elevations,
            capacities: M::from_element(8, 1, 1.0),
            sea_level: 0.5,
        };
        let mut actual = cloud();
        let mut expected = cloud();
        actual.position = v2(0, 0);
        expected.position = v2(0, 0);
        computer.compute_at(&mut actual);
        assert!(actual != expected); // Prevent evergreen test
        expected.evaporate(computer.params.evaporation_probability);
        assert_eq!(actual, expected);
    }

    #[test]
    fn compute_at_over_sea_level() {
        let elevations = M::from_vec(8, 1, vec![0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let computer = RainfallComputer {
            params: params(),
            elevations: &elevations,
            capacities: M::from_element(8, 1, 1.0),
            sea_level: 0.5,
        };
        let mut actual = cloud();
        let mut expected = cloud();
        actual.position = v2(1, 0);
        expected.position = v2(1, 0);
        actual.volume = 2.0;
        expected.volume = 2.0;
        computer.compute_at(&mut actual);
        assert!(actual != expected); // Prevent evergreen test
        expected.rain_over_capacity(computer.params.over_capacity_rain_probability);
        expected.rain_under_capacity(computer.params.under_capacity_rain_probability);
        assert_eq!(actual, expected);
    }

    #[test]
    fn valid_cloud() {
        let elevations = M::zeros(3, 3);
        let computer = RainfallComputer {
            params: params(),
            elevations: &elevations,
            capacities: M::zeros(3, 3),
            sea_level: 0.5,
        };
        assert!(computer.valid_cloud(&v2(0, 1), &v2(1, 0)));
    }

    #[test]
    fn cloud_not_valid_along_edge() {
        let elevations = M::zeros(3, 3);
        let computer = RainfallComputer {
            params: params(),
            elevations: &elevations,
            capacities: M::zeros(3, 3),
            sea_level: 0.5,
        };
        assert!(!computer.valid_cloud(&v2(0, 1), &v2(0, 1)));
    }

    #[test]
    fn cloud_valid_from_corner() {
        let elevations = M::zeros(3, 3);
        let computer = RainfallComputer {
            params: params(),
            elevations: &elevations,
            capacities: M::zeros(3, 3),
            sea_level: 0.5,
        };
        assert!(computer.valid_cloud(&v2(0, 0), &v2(0, 1)));
    }

    #[test]
    fn cloud_not_valid_heading_off_edge() {
        let elevations = M::zeros(3, 3);
        let computer = RainfallComputer {
            params: params(),
            elevations: &elevations,
            capacities: M::zeros(3, 3),
            sea_level: 0.5,
        };
        assert!(!computer.valid_cloud(&v2(0, 0), &v2(0, -1)));
    }

    #[test]
    fn compute_wind() {
        let elevations = M::from_vec(8, 1, vec![0.0, 1.0, 2.0, 0.0, 0.0, 1.0, 2.0, 1.0]);
        let params = RainfallParams {
            winds: RainfallParams::equal_probability_winds(),
            under_capacity_rain_probability: 0.1,
            over_capacity_rain_probability: 0.5,
            evaporation_probability: 0.1,
        };
        let computer = RainfallComputer {
            params,
            elevations: &elevations,
            capacities: M::from_vec(8, 1, vec![2.0, 1.0, 0.0, 2.0, 2.0, 1.0, 0.0, 1.0]),
            sea_level: 0.5,
        };
        let actual = computer.compute_wind(&Wind {
            direction: v2(1, 0),
            probability: 1.0,
        });
        let expected = M::from_vec(
            8,
            1,
            vec![0.0, 0.6, 0.7, 0.0, 0.0, 0.0947, 0.42615, 0.042615],
        );
        assert_eq!(actual, expected);
    }

}
