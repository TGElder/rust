use std::time::Duration;

use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::Vehicle;
use crate::bridge::{Bridge, BridgeType, InvalidBridge, Pier, Segment};
use crate::traits::has::HasParameters;
use crate::traits::{WithBridges, WithWorld};
use crate::world::World;

pub struct Crossings<T> {
    cx: T,
    one_cell_duration: Duration,
}

impl<T> Crossings<T>
where
    T: HasParameters + WithBridges + WithWorld,
{
    pub fn new(cx: T, one_cell_duration: Duration) -> Crossings<T> {
        Crossings {
            cx,
            one_cell_duration,
        }
    }

    pub async fn new_game(&self) {
        let crossings = self.get_crossings().await;
        let bridges = self.get_bridges(crossings).await;
        self.build_bridges(bridges).await;
    }

    async fn get_crossings(&self) -> Vec<[Pier; 3]> {
        let min_navigable_river_width = self.cx.parameters().npc_travel.min_navigable_river_width;
        let max_gradient = self.cx.parameters().npc_travel.max_walk_gradient;

        self.cx
            .with_world(|world| get_crossings(&world, &min_navigable_river_width, &max_gradient))
            .await
    }

    async fn get_bridges(&self, crossings: Vec<[Pier; 3]>) -> Vec<Bridge> {
        crossings
            .into_iter()
            .flat_map(|crossing| get_bridge(crossing, self.one_cell_duration))
            .collect()
    }

    async fn build_bridges(&self, to_build: Vec<Bridge>) {
        self.cx
            .mut_bridges(|bridges| {
                for bridge in to_build {
                    bridges
                        .entry(bridge.total_edge())
                        .or_default()
                        .insert(bridge);
                }
            })
            .await;
    }
}

fn get_crossings(
    world: &World,
    min_navigable_river_width: &f32,
    max_gradient: &f32,
) -> Vec<[Pier; 3]> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = v2(x, y);

            if let (Some(left), Some(right)) = (
                world.offset(&position, v2(-1, 0)),
                world.offset(&position, v2(1, 0)),
            ) {
                let horizontal = [left, position, right];
                if let Some(crossing) = is_crossing(
                    &world,
                    &min_navigable_river_width,
                    &max_gradient,
                    &horizontal,
                ) {
                    out.push(crossing);
                }
            }

            if let (Some(down), Some(up)) = (
                world.offset(&position, v2(0, -1)),
                world.offset(&position, v2(0, 1)),
            ) {
                let vertical = [down, position, up];
                if let Some(crossing) =
                    is_crossing(&world, &min_navigable_river_width, &max_gradient, &vertical)
                {
                    out.push(crossing);
                }
            }
        }
    }
    out
}

fn is_crossing(
    world: &World,
    min_navigable_river_width: &f32,
    max_gradient: &f32,
    positions: &[V2<usize>; 3],
) -> Option<[Pier; 3]> {
    if world.is_sea(&positions[0]) || world.is_sea(&positions[2]) {
        return None;
    }

    let cells = positions
        .iter()
        .flat_map(|position| world.get_cell(position))
        .collect::<Vec<_>>();

    if cells.len() != 3 {
        // At least one of the positions is out of bounds
        return None;
    }

    if cells[1].river.longest_side() < *min_navigable_river_width {
        return None;
    }

    if cells[0].elevation <= cells[1].elevation || cells[2].elevation <= cells[1].elevation {
        // Bridge is convex, meaning it will pass beneath terrain
        return None;
    }

    if cells[0].river.here() || cells[2].river.here() {
        return None;
    }

    if world.get_rise(&positions[0], &positions[1]).unwrap().abs() > *max_gradient {
        return None;
    }

    if world.get_rise(&positions[1], &positions[2]).unwrap().abs() > *max_gradient {
        return None;
    }

    Some([
        Pier {
            position: positions[0],
            elevation: world.get_cell_unsafe(&positions[0]).elevation,
            platform: true,
        },
        Pier {
            position: positions[1],
            elevation: world.get_cell_unsafe(&positions[1]).elevation,
            platform: false,
        },
        Pier {
            position: positions[2],
            elevation: world.get_cell_unsafe(&positions[2]).elevation,
            platform: true,
        },
    ])
}

fn get_bridge(crossing: [Pier; 3], duration: Duration) -> Result<Bridge, InvalidBridge> {
    Bridge {
        segments: vec![
            Segment {
                from: crossing[0],
                to: crossing[1],
                duration,
            },
            Segment {
                from: crossing[1],
                to: crossing[2],
                duration,
            },
        ],
        vehicle: Vehicle::None,
        bridge_type: BridgeType::Theoretical,
    }
    .validate()
}
