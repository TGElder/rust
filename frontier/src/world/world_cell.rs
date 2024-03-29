use super::climate::*;
use super::planned_road::*;
use super::world_object::*;
use commons::junction::*;
use commons::*;
use isometric::cell_traits::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct WorldCell {
    pub position: V2<usize>,
    pub elevation: f32,
    pub visible: bool,
    pub river: Junction,
    pub road: Junction,
    pub platform: Junction,
    pub planned_road: PlannedRoad,
    pub climate: Climate,
    pub object: WorldObject,
}

impl WorldCell {
    pub fn new(position: V2<usize>, elevation: f32) -> WorldCell {
        WorldCell {
            position,
            elevation,
            visible: false,
            river: Junction::default(),
            road: Junction::default(),
            platform: Junction::default(),
            planned_road: PlannedRoad::default(),
            climate: Climate::default(),
            object: WorldObject::None,
        }
    }
}

impl WithPosition for WorldCell {
    fn position(&self) -> V2<usize> {
        self.position
    }
}

impl WithElevation for WorldCell {
    fn elevation(&self) -> f32 {
        self.elevation
    }
}

impl WithVisibility for WorldCell {
    fn is_visible(&self) -> bool {
        self.visible
    }
}

impl WithJunction for WorldCell {
    fn junction(&self) -> Junction {
        fn merge(a: Junction1D, b: Junction1D, c: Junction1D) -> Junction1D {
            Junction1D {
                from: a.from || b.from || c.from,
                to: a.to || b.to || c.to,
                width: a.width.max(b.width).max(c.width),
            }
        }

        Junction {
            horizontal: merge(
                self.river.horizontal,
                self.road.horizontal,
                self.platform.horizontal,
            ),
            vertical: merge(
                self.river.vertical,
                self.road.vertical,
                self.platform.vertical,
            ),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_junction() {
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
        world_cell.platform.horizontal.to = true;
        world_cell.platform.horizontal.width = 3.0;
        assert_eq!(
            world_cell.junction(),
            Junction {
                horizontal: Junction1D {
                    width: 3.0,
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
}
