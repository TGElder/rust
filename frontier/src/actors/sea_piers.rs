use std::time::Duration;

use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::Vehicle;
use crate::bridge::{Bridge, BridgeType, InvalidBridge, Pier, Segment};
use crate::traits::has::HasParameters;
use crate::traits::{WithBridges, WithWorld};
use crate::world::World;

pub struct SeaPiers<T> {
    cx: T,
    one_cell_duration: Duration,
}

impl<T> SeaPiers<T>
where
    T: HasParameters + WithBridges + WithWorld + Sync,
{
    pub fn new(cx: T, one_cell_duration: Duration) -> SeaPiers<T> {
        SeaPiers {
            cx,
            one_cell_duration,
        }
    }

    pub async fn new_game(&self) {
        let piers = self.get_piers().await;
        let bridges = self.get_bridges(piers).await;
        self.build_bridges(bridges).await;
    }

    async fn get_piers(&self) -> Vec<Vec<Segment>> {
        self.cx
            .with_world(|world| get_piers(&world, &self.one_cell_duration))
            .await
    }

    async fn get_bridges(&self, piers: Vec<Vec<Segment>>) -> Vec<Bridge> {
        piers.into_iter().flat_map(get_bridge).collect()
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

fn get_piers(world: &World, duration: &Duration) -> Vec<Vec<Segment>> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            let from = v2(x, y);

            if let Some(to) = world.offset(&from, v2(1, 0)) {
                if let Some(pier) = is_pier(&world, &from, &to, duration) {
                    out.push(pier);
                }
            }

            if let Some(to) = world.offset(&from, v2(0, 1)) {
                if let Some(pier) = is_pier(&world, &from, &to, duration) {
                    out.push(pier);
                }
            }
        }
    }
    out
}

fn is_pier(
    world: &World,
    from: &V2<usize>,
    to: &V2<usize>,
    duration: &Duration,
) -> Option<Vec<Segment>> {
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

    Some(vec![
        Segment {
            from: Pier {
                position: *from,
                elevation: from_elevation,
                platform: true,
            },
            to: Pier {
                position: *to,
                elevation: to_elevation,
                platform: false,
            },
            duration: *duration,
        },
        Segment {
            from: Pier {
                position: *to,
                elevation: to_elevation,
                platform: false,
            },
            to: Pier {
                position: *to,
                elevation: sea_level,
                platform: false,
            },
            duration: Duration::from_millis(0),
        },
    ])
}

fn get_bridge(segments: Vec<Segment>) -> Result<Bridge, InvalidBridge> {
    Bridge {
        segments,
        vehicle: Vehicle::None,
        bridge_type: BridgeType::Theoretical,
    }
    .validate()
}
