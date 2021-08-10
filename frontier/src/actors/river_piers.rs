use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::Vehicle;
use crate::bridges::{Bridge, BridgeType, Pier, Segment};
use crate::traits::has::HasParameters;
use crate::traits::{WithBridges, WithWorld};
use crate::world::World;

pub struct RiverPiers<T> {
    cx: T,
}

impl<T> RiverPiers<T>
where
    T: HasParameters + WithBridges + WithWorld + Sync,
{
    pub fn new(cx: T) -> RiverPiers<T> {
        RiverPiers { cx }
    }

    pub async fn new_game(&self) {
        let segments = self.get_segments().await;
        let bridges = self.get_bridges(segments).await;
        self.build_bridges(bridges).await;
    }

    async fn get_segments(&self) -> Vec<Vec<Segment>> {
        let min_navigable_river_width = self.cx.parameters().npc_travel.min_navigable_river_width;
        self.cx
            .with_world(|world| get_segments(&world, &min_navigable_river_width))
            .await
    }

    async fn get_bridges(&self, segments: Vec<Vec<Segment>>) -> Vec<Bridge> {
        segments
            .into_iter()
            .flat_map(|segments| {
                Bridge {
                    segments,
                    vehicle: Vehicle::None,
                    bridge_type: BridgeType::Theoretical,
                }
                .validate()
            })
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

fn get_segments(world: &World, min_navigable_river_width: &f32) -> Vec<Vec<Segment>> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            for offset in [v2(1, 0), v2(0, 1)].iter() {
                let from = v2(x, y);

                if let Some(to) = world.offset(&from, *offset) {
                    if let Some(pier) = is_pier(&world, &from, &to, min_navigable_river_width) {
                        out.push(pier);
                    }
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
    min_navigable_river_width: &f32,
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

    if to_cell.river.longest_side() < *min_navigable_river_width {
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
        },
        Segment {
            from: Pier {
                position: *to,
                elevation: to_elevation,
                platform: false,
            },
            to: Pier {
                position: *to,
                elevation: to_elevation,
                platform: false,
            },
        },
    ])
}
