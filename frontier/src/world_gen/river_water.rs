use super::*;
use crate::world::*;
use commons::*;
use isometric::cell_traits::*;
use std::default::Default;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct RiverWaterParams {
    pub max_distance_as_world_pc: f32,
}

impl Default for RiverWaterParams {
    fn default() -> RiverWaterParams {
        RiverWaterParams {
            max_distance_as_world_pc: 0.01,
        }
    }
}

pub fn compute_river_water(world: &World, params: &WorldGenParameters) -> M<f32> {
    let threshold = get_threshold(
        params.river_width_range.1 as f32,
        world.width() as f32 * params.river_water.max_distance_as_world_pc,
    );
    let mut river_water = Riverwater::new(threshold, world);
    river_water.compute();
    rescale_ignoring_sea(river_water.result, world)
}

pub fn load_river_water(world: &mut World, river_water: &M<f32>) {
    for x in 0..river_water.width() {
        for y in 0..river_water.height() {
            world.mut_cell_unsafe(&v2(x, y)).climate.river_water = river_water[(x, y)] as f32;
        }
    }
}

struct Riverwater<'a> {
    threshold: f32,
    world: &'a World,
    result: M<f32>,
}

impl<'a> Riverwater<'a> {
    fn new(threshold: f32, world: &'a World) -> Riverwater<'a> {
        Riverwater {
            threshold,
            world,
            result: M::zeros(world.width(), world.height()),
        }
    }

    fn add_river_water_at_position(&mut self, cell: &WorldCell) {
        let flow = cell.river.width().max(cell.river.height());
        let max = get_max_distance(self.threshold, flow);
        for dx in -max..=max {
            for dy in -max..=max {
                if let Some(other) = self.world.offset(&cell.position(), &v2(dx, dy)) {
                    let other_cell = self.world.get_cell_unsafe(&other);
                    let dz = other_cell.elevation - cell.elevation;
                    let river_water = river_water(flow, v3(dx as f32, dy as f32, dz));
                    if river_water >= self.threshold {
                        *self.result.mut_cell_unsafe(&other) += river_water;
                    }
                }
            }
        }
    }

    fn compute(&mut self) {
        for x in 0..self.world.width() {
            for y in 0..self.world.height() {
                let cell = self.world.get_cell_unsafe(&v2(x, y));
                if cell.river.here() {
                    self.add_river_water_at_position(self.world.get_cell_unsafe(&v2(x, y)));
                }
            }
        }
    }
}

fn get_threshold(max_flow: f32, max_distance: f32) -> f32 {
    max_flow / (max_distance + 1.0)
}

fn get_max_distance(threshold: f32, flow: f32) -> i32 {
    (flow / threshold).floor() as i32 - 1
}

fn river_water(flow: f32, delta: V3<f32>) -> f32 {
    let distance = delta.magnitude();
    flow / (distance + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::junction::*;

    #[test]
    fn test_get_max_distance() {
        assert_eq!(get_max_distance(0.05, 0.55), 10);
    }

    #[test]
    fn test_get_max_distance_low_flow() {
        assert_eq!(get_max_distance(0.05, 0.01), -1);
    }

    #[test]
    fn test_get_river_water_origin() {
        assert_eq!(river_water(11.0, v3(0.0, 0.0, 0.0)), 11.0);
    }

    #[test]
    fn test_get_river_water_other() {
        assert_eq!(river_water(11.0, v3(4.0, 0.0, 0.0)), 2.2);
    }

    #[test]
    #[rustfmt::skip]
    fn test_add_river_water_for() {
        let mut world = World::new(
            M::zeros(5, 5),
            0.5
        );
        let mut river = PositionJunction::new(v2(2, 2));
        river.junction.horizontal.width = 0.3;
        river.junction.vertical.width = 0.2;
        world.add_river(river);

        let mut river_water = Riverwater::new(0.1, &world);
        river_water.add_river_water_at_position(&world.get_cell_unsafe(&v2(2, 2)));

        let actual = river_water.result.map(|v| (v * 100.0).floor() / 100.0);

        let expected = M::from_vec(5, 5, vec![
            0.0, 0.0, 0.1, 0.0, 0.0,
            0.0, 0.12, 0.15, 0.12, 0.0,
            0.1, 0.15, 0.3, 0.15, 0.1,
            0.0, 0.12, 0.15, 0.12, 0.0,
            0.0, 0.0, 0.1, 0.0, 0.0,
        ]).transpose();

        assert_eq!(actual, expected);
        
    }

    #[test]
    #[rustfmt::skip]
    fn test_add_river_water_cumulative() {
        let mut world = World::new(
            M::zeros(5, 5),
            0.5
        );
        let mut river = PositionJunction::new(v2(2, 2));
        river.junction.horizontal.width = 0.3;
        river.junction.vertical.width = 0.2;
        world.add_river(river);

        let mut river_water = Riverwater::new(0.1, &world);
        river_water.add_river_water_at_position(&world.get_cell_unsafe(&v2(2, 2)));
        river_water.add_river_water_at_position(&world.get_cell_unsafe(&v2(2, 2)));

        let actual = river_water.result.map(|v| (v * 100.0).floor() / 100.0);

        let expected = M::from_vec(5, 5, vec![
            0.0, 0.0, 0.2, 0.0, 0.0,
            0.0, 0.24, 0.3, 0.24, 0.0,
            0.2, 0.3, 0.6, 0.3, 0.2,
            0.0, 0.24, 0.3, 0.24, 0.0,
            0.0, 0.0, 0.2, 0.0, 0.0,
        ]).transpose();

        assert_eq!(actual, expected);
        
    }

}
