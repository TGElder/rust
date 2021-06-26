use std::time::Duration;

use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::Vehicle;
use crate::bridge::{Bridge, BridgeType, InvalidBridge};
use crate::traits::has::HasParameters;
use crate::traits::{WithBridges, WithWorld};
use crate::travel_duration::EdgeDuration;
use crate::world::World;

pub struct Crossings<T> {
    cx: T,
    edge_duration: Duration,
}

impl<T> Crossings<T>
where
    T: HasParameters + WithBridges + WithWorld,
{
    pub fn new(cx: T, edge_duration: Duration) -> Crossings<T> {
        Crossings { cx, edge_duration }
    }

    pub async fn new_game(&self) {
        let crossings = self.get_crossings().await;
        let bridges = self.get_bridges(crossings).await;
        self.build_bridges(bridges).await;
    }

    async fn get_crossings(&self) -> Vec<[V2<usize>; 3]> {
        let min_navigable_river_width = self.cx.parameters().npc_travel.min_navigable_river_width;

        self.cx
            .with_world(|world| get_crossings(&world, &min_navigable_river_width))
            .await
    }

    async fn get_bridges(&self, crossings: Vec<[V2<usize>; 3]>) -> Vec<Bridge> {
        crossings
            .into_iter()
            .flat_map(|crossing| get_bridge(crossing, self.edge_duration))
            .collect()
    }

    async fn build_bridges(&self, to_build: Vec<Bridge>) {
        self.cx
            .mut_bridges(|bridges| {
                for bridge in to_build {
                    bridges.insert(bridge.total_edge(), bridge);
                }
            })
            .await;
    }
}

fn get_crossings(world: &World, min_navigable_river_width: &f32) -> Vec<[V2<usize>; 3]> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = v2(x, y);

            if let (Some(left), Some(right)) = (
                world.offset(&position, v2(-1, 0)),
                world.offset(&position, v2(1, 0)),
            ) {
                let horizontal = [left, position, right];
                if is_crossing(&world, &min_navigable_river_width, &horizontal) {
                    out.push(horizontal);
                }
            }

            if let (Some(down), Some(up)) = (
                world.offset(&position, v2(0, -1)),
                world.offset(&position, v2(0, 1)),
            ) {
                let vertical = [down, position, up];
                if is_crossing(&world, &min_navigable_river_width, &vertical) {
                    out.push(vertical);
                }
            }
        }
    }
    out
}

fn is_crossing(world: &World, min_navigable_river_width: &f32, positions: &[V2<usize>; 3]) -> bool {
    if world.is_sea(&positions[0]) || world.is_sea(&positions[2]) {
        return false;
    }

    let cells = positions
        .iter()
        .flat_map(|position| world.get_cell(position))
        .collect::<Vec<_>>();

    if cells.len() != 3 {
        // At least one of the cells is invalid
        return false;
    }

    if cells[0].river.here() || cells[2].river.here() {
        return false;
    }

    cells[1].river.longest_side() >= *min_navigable_river_width
}

fn get_bridge(crossing: [V2<usize>; 3], duration: Duration) -> Result<Bridge, InvalidBridge> {
    Bridge::new(
        vec![
            EdgeDuration {
                from: crossing[0],
                to: crossing[1],
                duration: Some(duration),
            },
            EdgeDuration {
                from: crossing[1],
                to: crossing[2],
                duration: Some(duration),
            },
        ],
        Vehicle::None,
        BridgeType::Theoretical,
    )
}
