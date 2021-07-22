use std::time::Duration;

use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::Vehicle;
use crate::bridge::{Bridge, BridgeType, InvalidBridge, Pier, Segment};
use crate::traits::has::HasParameters;
use crate::traits::{WithBridges, WithWorld};
use crate::world::World;

pub struct Piers<T> {
    cx: T,
    one_cell_duration: Duration,
}

impl<T> Piers<T>
where
    T: HasParameters + WithBridges + WithWorld,
{
    pub fn new(cx: T, one_cell_duration: Duration) -> Piers<T> {
        Piers {
            cx,
            one_cell_duration,
        }
    }

    pub async fn new_game(&self) {
        let piers = self.get_piers().await;
        let bridges = self.get_bridges(piers).await;
        self.build_bridges(bridges).await;
    }

    async fn get_piers(&self) -> Vec<[Pier; 2]> {
        self.cx.with_world(|world| get_piers(&world)).await
    }

    async fn get_bridges(&self, piers: Vec<[Pier; 2]>) -> Vec<Bridge> {
        piers
            .into_iter()
            .flat_map(|pier| get_bridge(pier, self.one_cell_duration))
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

fn get_piers(world: &World) -> Vec<[Pier; 2]> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            let from = v2(x, y);

            if let Some(to) = world.offset(&from, v2(1, 0)) {
                if let Some(pier) = is_pier(&world, &from, &to) {
                    out.push(pier);
                }
            }

            if let Some(to) = world.offset(&from, v2(0, 1)) {
                if let Some(pier) = is_pier(&world, &from, &to) {
                    out.push(pier);
                }
            }
        }
    }
    out
}

fn is_pier(world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<[Pier; 2]> {
    let from_cell = world.get_cell_unsafe(from);
    let to_cell = world.get_cell_unsafe(to);

    let from_elevation = from_cell.elevation;
    let to_elevation = to_cell.elevation;

    let sea_level = world.sea_level();

    if from_cell.river.here() {
        return None;
    }

    if from_elevation <= sea_level {
        return None;
    }

    if to_elevation > sea_level {
        return None;
    }

    Some([
        Pier {
            position: *from,
            elevation: from_elevation,
            platform: true,
        },
        Pier {
            position: *to,
            elevation: to_elevation,
            platform: false,
        },
    ])
}

fn get_bridge(pier: [Pier; 2], duration: Duration) -> Result<Bridge, InvalidBridge> {
    Bridge {
        segments: vec![Segment {
            from: pier[0],
            to: pier[1],
            duration,
        }],
        vehicle: Vehicle::None,
        bridge_type: BridgeType::Theoretical,
    }
    .validate()
}
