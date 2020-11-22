mod climate;
mod planned_road;
mod vegetation;
mod world_cell;
mod world_object;

pub use climate::*;
pub use planned_road::*;
pub use vegetation::*;
pub use world_cell::*;
pub use world_object::*;

use commons::edge::*;
use commons::junction::*;
use commons::unsafe_ordering;
use commons::*;
use isometric::cell_traits::*;
use isometric::coords::WorldCoord;
use serde::{Deserialize, Serialize};

const ROAD_WIDTH: f32 = 0.05;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    width: usize,
    height: usize,
    cells: M<WorldCell>,
    sea_level: f32,
    max_height: f32,
}

impl Grid<WorldCell> for World {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn in_bounds(&self, position: &V2<usize>) -> bool {
        position.x < self.width() && position.y < self.height()
    }

    fn get_cell_unsafe(&self, position: &V2<usize>) -> &WorldCell {
        self.cells.get_cell_unsafe(position)
    }

    fn mut_cell_unsafe(&mut self, position: &V2<usize>) -> &mut WorldCell {
        self.cells.mut_cell_unsafe(position)
    }
}

impl World {
    pub fn new(elevations: M<f32>, sea_level: f32) -> World {
        let (width, height) = elevations.shape();
        let max_height = elevations.max();
        World {
            width,
            height,
            cells: M::from_fn(width, height, |x, y| {
                WorldCell::new(v2(x, y), elevations[(x, y)])
            }),
            sea_level,
            max_height,
        }
    }

    pub fn sea_level(&self) -> f32 {
        self.sea_level
    }

    pub fn max_height(&self) -> f32 {
        self.max_height
    }

    pub fn add_river<T>(&mut self, cell: T)
    where
        T: WithPosition + WithJunction,
    {
        self.mut_cell_unsafe(&cell.position()).river = cell.junction();
    }

    pub fn set_road(&mut self, road: &Edge, state: bool) {
        let set_width = |junction_1d: &mut Junction1D| {
            junction_1d.width = if junction_1d.from || junction_1d.to {
                ROAD_WIDTH
            } else {
                0.0
            }
        };
        let from = self.mut_cell_unsafe(road.from());
        let from_junction_1d = from.road.junction_1d(road.horizontal());
        from_junction_1d.from = state;
        set_width(from_junction_1d);
        let to = self.mut_cell_unsafe(road.to());
        let to_junction_1d = to.road.junction_1d(road.horizontal());
        to_junction_1d.to = state;
        set_width(to_junction_1d);
    }

    pub fn plan_road(&mut self, road: &Edge, when: Option<u128>) {
        let from = self.mut_cell_unsafe(road.from());
        from.planned_road.get_mut(road.horizontal()).from = when;

        let to = self.mut_cell_unsafe(road.to());
        to.planned_road.get_mut(road.horizontal()).to = when;
    }

    pub fn is_sea(&self, position: &V2<usize>) -> bool {
        self.get_cell(position)
            .map(|cell| cell.elevation())
            .map(|elevation| elevation <= self.sea_level)
            .unwrap_or(false)
    }

    fn is(&self, edge: &Edge, junction_fn: &dyn Fn(&WorldCell) -> Junction) -> bool {
        if let Some(cell) = self.get_cell(&edge.from()) {
            let junction = junction_fn(cell);
            if edge.horizontal() {
                junction.horizontal.from
            } else {
                junction.vertical.from
            }
        } else {
            false
        }
    }

    pub fn is_road(&self, edge: &Edge) -> bool {
        self.is(edge, &|cell| cell.road)
    }

    pub fn road_planned(&self, edge: &Edge) -> Option<u128> {
        let cell = self.get_cell(&edge.from())?;
        cell.planned_road.get(edge.horizontal()).from
    }

    pub fn is_river(&self, edge: &Edge) -> bool {
        self.is(edge, &|cell| cell.river)
    }

    pub fn is_river_or_road(&self, edge: &Edge) -> bool {
        self.is(edge, &|cell| cell.junction())
    }

    pub fn reveal_all(&mut self) {
        for x in 0..self.width {
            for y in 0..self.height {
                self.mut_cell_unsafe(&v2(x, y)).visible = true;
            }
        }
    }

    pub fn snap(&self, world_coord: WorldCoord) -> WorldCoord {
        let z = if let Some(cell) = self.get_cell(&world_coord.to_v2_round()) {
            cell.elevation()
        } else {
            world_coord.z
        };
        WorldCoord::new(world_coord.x.round(), world_coord.y.round(), z)
    }

    #[allow(clippy::many_single_char_names)]
    pub fn snap_to_edge(&self, position: &V2<f32>) -> f32 {
        let x = position.x;
        let y = position.y;
        let (a, b, p) = if x.fract() == 0.0 {
            (
                v2(x as usize, y.floor() as usize),
                v2(x as usize, y.ceil() as usize),
                y.fract(),
            )
        } else if y.fract() == 0.0 {
            (
                v2(x.floor() as usize, y as usize),
                v2(x.ceil() as usize, y as usize),
                x.fract(),
            )
        } else {
            panic!(
                "Trying to snap x={}, y={} to line. One of x or y must be a whole number.",
                x, y
            );
        };
        let a = self.get_cell(&a).unwrap().elevation();
        let b = self.get_cell(&b).unwrap().elevation();
        (b - a) * p + a
    }

    pub fn snap_to_middle(&self, position: &V2<f32>) -> Option<f32> {
        let x = position.x.floor();
        let y = position.y.floor();
        if let (Some(a), Some(b)) = (
            self.get_cell(&v2(x as usize, y as usize)),
            self.get_cell(&v2(x as usize + 1, y as usize + 1)),
        ) {
            let z = (a.elevation + b.elevation) / 2.0;
            Some(z)
        } else {
            None
        }
    }

    pub fn get_border(&self, position: &V2<usize>) -> Vec<Edge> {
        let corners = get_corners(position);
        (0..4)
            .map(|i| Edge::new(corners[i], corners[(i + 1) % 4]))
            .collect()
    }

    pub fn get_rise(&self, from: &V2<usize>, to: &V2<usize>) -> Option<f32> {
        match (self.get_cell(from), self.get_cell(to)) {
            (Some(from), Some(to)) => Some(to.elevation() - from.elevation()),
            _ => None,
        }
    }

    pub fn get_lowest_corner(&self, position: &V2<usize>) -> f32 {
        get_corners(&position)
            .iter()
            .flat_map(|corner| self.get_cell(corner))
            .map(|cell| cell.elevation())
            .min_by(unsafe_ordering)
            .unwrap()
    }

    #[allow(dead_code)]
    pub fn get_highest_corner(&self, position: &V2<usize>) -> f32 {
        get_corners(&position)
            .iter()
            .flat_map(|corner| self.get_cell(corner))
            .map(|cell| cell.elevation())
            .max_by(unsafe_ordering)
            .unwrap()
    }

    pub fn get_max_abs_rise(&self, position: &V2<usize>) -> f32 {
        self.get_border(&position)
            .iter()
            .flat_map(|edge| self.get_rise(&edge.from(), &edge.to()))
            .map(|rise| rise.abs())
            .max_by(unsafe_ordering)
            .unwrap()
    }

    pub fn tile_average(
        &self,
        position: &V2<usize>,
        function: &dyn Fn(&WorldCell) -> Option<f32>,
    ) -> Option<f32> {
        let values: Vec<f32> = self
            .get_corners_in_bounds(&position)
            .iter()
            .map(|p| function(self.get_cell_unsafe(p)))
            .flatten()
            .collect();
        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum::<f32>() / (values.iter().count() as f32))
        }
    }

    pub fn tile_avg_groundwater(&self, position: &V2<usize>) -> Option<f32> {
        self.tile_average(&position, &|cell| {
            if !self.is_sea(&cell.position) {
                Some(cell.climate.groundwater)
            } else {
                None
            }
        })
    }

    pub fn tile_avg_temperature(&self, position: &V2<usize>) -> Option<f32> {
        self.tile_average(&position, &|cell| {
            if !self.is_sea(&cell.position) {
                Some(cell.climate.temperature)
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::almost::Almost;

    #[rustfmt::skip]
    fn world() -> World {
        let mut out = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 2.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            0.5,
        );
        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.vertical.from = true;
        river_1.junction.vertical.width = 0.1;
        let mut river_2 = PositionJunction::new(v2(1, 1));
        river_2.junction.vertical.to = true;
        river_2.junction.vertical.from = true;
        river_2.junction.vertical.width = 0.2;
        let mut river_3 = PositionJunction::new(v2(1, 2));
        river_3.junction.vertical.to = true;
        river_3.junction.vertical.width = 0.3;
        river_3.junction.horizontal.from = true;
        river_3.junction.horizontal.width = 0.3;
        let mut river_4 = PositionJunction::new(v2(2, 2));
        river_4.junction.vertical.to = true;
        river_4.junction.horizontal.width = 0.4;

        out.add_river(river_1);
        out.add_river(river_2);
        out.add_river(river_3);
        out.add_river(river_4);

        out
    }

    #[test]
    fn test_world_cell_junction() {
        let mut world_cell = WorldCell::new(v2(0, 0), 0.0);
        assert_eq!(world_cell.junction(), Junction::default());
        world_cell.river.horizontal.from = true;
        world_cell.river.horizontal.width = 1.0;
        world_cell.road.vertical.to = true;
        world_cell.road.vertical.width = 2.0;
        assert_eq!(
            world_cell.junction(),
            Junction {
                horizontal: Junction1D {
                    width: 1.0,
                    from: true,
                    to: false,
                },
                vertical: Junction1D {
                    width: 2.0,
                    from: false,
                    to: true,
                },
            }
        );
        world_cell.road.horizontal.to = true;
        world_cell.road.horizontal.width = 2.0;
        assert_eq!(
            world_cell.junction(),
            Junction {
                horizontal: Junction1D {
                    width: 2.0,
                    from: true,
                    to: true,
                },
                vertical: Junction1D {
                    width: 2.0,
                    from: false,
                    to: true,
                },
            }
        );
    }

    #[test]
    fn test_snap() {
        assert_eq!(
            world().snap(WorldCoord::new(0.3, 1.7, 1.2)),
            WorldCoord::new(0.0, 2.0, 1.0)
        );
    }

    #[test]
    fn test_snap_to_edge_x() {
        assert!(world().snap_to_edge(&v2(0.3, 1.0)).almost(&1.3));
    }

    #[test]
    fn test_snap_to_edge_y() {
        assert!(world().snap_to_edge(&v2(1.0, 1.6)).almost(&1.4));
    }

    #[test]
    fn test_snap_to_middle() {
        assert_eq!(world().snap_to_middle(&v2(0.3, 0.7)), Some(1.5));
        assert_eq!(world().snap_to_middle(&v2(3.3, 1.7)), None);
    }

    #[test]
    fn test_get_border() {
        assert_eq!(
            world().get_border(&v2(0, 0)),
            vec![
                Edge::new(v2(0, 0), v2(1, 0)),
                Edge::new(v2(1, 0), v2(1, 1)),
                Edge::new(v2(1, 1), v2(0, 1)),
                Edge::new(v2(0, 1), v2(0, 0)),
            ]
        );
    }

    #[test]
    fn test_in_bounds() {
        assert!(world().in_bounds(&v2(1, 1)));
        assert!(!world().in_bounds(&v2(3, 1)));
        assert!(!world().in_bounds(&v2(1, 3)));
        assert!(!world().in_bounds(&v2(3, 3)));
    }

    #[test]
    fn test_is_sea() {
        let world = World::new(M::from_vec(2, 1, vec![1.0, 0.0]), 0.5);
        assert!(!world.is_sea(&v2(0, 0)));
        assert!(world.is_sea(&v2(1, 0)));
    }

    #[test]
    fn test_is_road_horizontal() {
        let mut world = world();
        world.set_road(&Edge::new(v2(0, 0), v2(0, 1)), true);
        assert!(world.is_road(&Edge::new(v2(0, 0), v2(0, 1))));
        assert!(world.is_road(&Edge::new(v2(0, 1), v2(0, 0))));
        assert!(world.get_cell_unsafe(&v2(0, 0)).road.here());
        assert!(world.get_cell_unsafe(&v2(0, 1)).road.here());
        assert!(!world.is_road(&Edge::new(v2(1, 0), v2(1, 1))));
        assert!(!world.is_road(&Edge::new(v2(0, 1), v2(0, 2))));
    }

    #[test]
    fn test_is_road_vertical() {
        let mut world = world();
        world.set_road(&Edge::new(v2(0, 0), v2(1, 0)), true);
        assert!(world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
        assert!(world.is_road(&Edge::new(v2(1, 0), v2(0, 0))));
        assert!(world.get_cell_unsafe(&v2(0, 0)).road.here());
        assert!(world.get_cell_unsafe(&v2(1, 0)).road.here());
        assert!(!world.is_road(&Edge::new(v2(1, 0), v2(2, 0))));
        assert!(!world.is_road(&Edge::new(v2(0, 1), v2(1, 1))));
    }

    #[test]
    fn test_road_planned_horizontal() {
        let mut world = world();
        assert_eq!(world.road_planned(&Edge::new(v2(0, 0), v2(0, 1))), None);
        assert_eq!(world.road_planned(&Edge::new(v2(0, 1), v2(0, 0))), None);
        world.plan_road(&Edge::new(v2(0, 0), v2(0, 1)), Some(123));
        assert_eq!(
            world.road_planned(&Edge::new(v2(0, 0), v2(0, 1))),
            Some(123)
        );
        assert_eq!(
            world.road_planned(&Edge::new(v2(0, 1), v2(0, 0))),
            Some(123)
        );
    }

    #[test]
    fn test_road_planned_vertical() {
        let mut world = world();
        assert_eq!(world.road_planned(&Edge::new(v2(0, 0), v2(1, 0))), None);
        assert_eq!(world.road_planned(&Edge::new(v2(1, 0), v2(0, 0))), None);
        world.plan_road(&Edge::new(v2(0, 0), v2(1, 0)), Some(123));
        assert_eq!(
            world.road_planned(&Edge::new(v2(0, 0), v2(1, 0))),
            Some(123)
        );
        assert_eq!(
            world.road_planned(&Edge::new(v2(1, 0), v2(0, 0))),
            Some(123)
        );
    }

    #[test]
    fn test_road_planned_horizontal_and_vertical() {
        let mut world = world();
        assert_eq!(world.road_planned(&Edge::new(v2(0, 0), v2(1, 0))), None);
        assert_eq!(world.road_planned(&Edge::new(v2(1, 0), v2(0, 0))), None);
        assert_eq!(world.road_planned(&Edge::new(v2(0, 0), v2(0, 1))), None);
        assert_eq!(world.road_planned(&Edge::new(v2(0, 1), v2(0, 0))), None);
        world.plan_road(&Edge::new(v2(0, 0), v2(1, 0)), Some(123));
        world.plan_road(&Edge::new(v2(0, 0), v2(0, 1)), Some(123));
        assert_eq!(
            world.road_planned(&Edge::new(v2(0, 0), v2(1, 0))),
            Some(123)
        );
        assert_eq!(
            world.road_planned(&Edge::new(v2(1, 0), v2(0, 0))),
            Some(123)
        );
        assert_eq!(
            world.road_planned(&Edge::new(v2(0, 0), v2(0, 1))),
            Some(123)
        );
        assert_eq!(
            world.road_planned(&Edge::new(v2(0, 1), v2(0, 0))),
            Some(123)
        );
    }

    #[test]
    fn test_is_river() {
        let mut world = world();
        world.set_road(&Edge::new(v2(0, 0), v2(0, 1)), true);
        assert!(world.is_river(&Edge::new(v2(1, 0), v2(1, 1))));
        assert!(!world.is_river(&Edge::new(v2(0, 1), v2(0, 2))));
    }

    #[test]
    fn test_is_river_or_road() {
        let mut world = world();
        world.set_road(&Edge::new(v2(0, 0), v2(0, 1)), true);
        assert!(world.is_river_or_road(&Edge::new(v2(0, 0), v2(0, 1))));
        assert!(world.is_river_or_road(&Edge::new(v2(1, 0), v2(1, 1))));
        assert!(!world.is_river_or_road(&Edge::new(v2(0, 1), v2(0, 2))));
    }

    #[test]
    fn test_get_rise() {
        assert!(world().get_rise(&v2(1, 0), &v2(1, 1)).unwrap().almost(&1.0));
        assert!(world()
            .get_rise(&v2(1, 1), &v2(2, 1))
            .unwrap()
            .almost(&-1.0));
        assert!(world().get_rise(&v2(0, 0), &v2(1, 0)).unwrap().almost(&0.0));
    }

    #[test]
    fn test_get_lowest_corner() {
        assert!(world().get_lowest_corner(&v2(0, 0)).almost(&1.0));
    }

    #[test]
    fn test_get_highest_corner() {
        assert!(world().get_highest_corner(&v2(0, 0)).almost(&2.0));
    }

    #[test]
    fn test_get_max_abs_rise() {
        assert!(world().get_max_abs_rise(&v2(0, 0)).almost(&1.0));
    }

    #[test]
    fn test_set_visible() {
        let mut world = world();
        assert!(!world.get_cell(&v2(0, 0)).unwrap().is_visible());
        world.mut_cell_unsafe(&v2(0, 0)).visible = true;
        assert!(world.get_cell(&v2(0, 0)).unwrap().is_visible());
    }

    #[test]
    fn test_reveal_all() {
        let mut world = world();
        assert!(!world.get_cell(&v2(0, 0)).unwrap().is_visible());
        assert!(!world.get_cell(&v2(0, 0)).unwrap().is_visible());
        assert!(!world.get_cell(&v2(0, 0)).unwrap().is_visible());
        assert!(!world.get_cell(&v2(0, 1)).unwrap().is_visible());
        assert!(!world.get_cell(&v2(1, 1)).unwrap().is_visible());
        assert!(!world.get_cell(&v2(2, 1)).unwrap().is_visible());
        assert!(!world.get_cell(&v2(0, 2)).unwrap().is_visible());
        assert!(!world.get_cell(&v2(1, 2)).unwrap().is_visible());
        assert!(!world.get_cell(&v2(2, 2)).unwrap().is_visible());
        world.reveal_all();
        assert!(world.get_cell(&v2(0, 0)).unwrap().is_visible());
        assert!(world.get_cell(&v2(0, 0)).unwrap().is_visible());
        assert!(world.get_cell(&v2(0, 0)).unwrap().is_visible());
        assert!(world.get_cell(&v2(0, 1)).unwrap().is_visible());
        assert!(world.get_cell(&v2(1, 1)).unwrap().is_visible());
        assert!(world.get_cell(&v2(2, 1)).unwrap().is_visible());
        assert!(world.get_cell(&v2(0, 2)).unwrap().is_visible());
        assert!(world.get_cell(&v2(1, 2)).unwrap().is_visible());
        assert!(world.get_cell(&v2(2, 2)).unwrap().is_visible());
    }

    #[test]
    fn test_simple_tile_average() {
        let world = world();
        let actual = world.tile_average(&v2(0, 0), &|_| Some(1.0));
        assert_eq!(actual, Some(1.0));
    }

    #[test]
    fn test_filtered_tile_average() {
        let world = world();
        let actual = world.tile_average(&v2(0, 0), &|cell| {
            if cell.elevation <= 1.0 {
                Some(cell.elevation)
            } else {
                None
            }
        });
        assert_eq!(actual, Some(1.0));
    }

    #[test]
    fn test_all_none_tile_average() {
        let world = world();
        let actual = world.tile_average(&v2(0, 0), &|_| None);
        assert_eq!(actual, None);
    }

    #[test]
    fn test_tile_groundwater_average() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(0, 0)).climate.groundwater = 1.0;
        world.mut_cell_unsafe(&v2(1, 1)).climate.groundwater = 3.0;
        let actual = world.tile_avg_groundwater(&v2(0, 0));
        assert_eq!(actual, Some(1.0)); // ( 1 + 0 + 3 + 0 ) / 4
    }

    #[test]
    fn test_tile_temperature_average() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(0, 0)).climate.temperature = 10.0;
        world.mut_cell_unsafe(&v2(1, 1)).climate.temperature = 30.0;
        let actual = world.tile_avg_temperature(&v2(0, 0));
        assert_eq!(actual, Some(10.0)); // ( 10 + 0 + 30 + 0 ) / 4
    }

    #[test]
    fn round_trip() {
        let original = world();
        let encoded: Vec<u8> = bincode::serialize(&original).unwrap();
        let reconstructed: World = bincode::deserialize(&encoded[..]).unwrap();
        assert_eq!(original, reconstructed);
    }
}
