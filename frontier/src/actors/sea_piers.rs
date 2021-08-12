use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::Vehicle;
use crate::bridges::{Bridge, BridgeType, Pier};
use crate::traits::has::HasParameters;
use crate::traits::{WithBridges, WithWorld};
use crate::world::World;

pub struct SeaPiers<T> {
    cx: T,
    parameters: SeaPierParameters,
}

pub struct SeaPierParameters {
    pub deep_sea_level: f32,
}

impl<T> SeaPiers<T>
where
    T: HasParameters + WithBridges + WithWorld + Sync,
{
    pub fn new(cx: T, parameters: SeaPierParameters) -> SeaPiers<T> {
        SeaPiers { cx, parameters }
    }

    pub async fn new_game(&self) {
        let piers = self.get_piers().await;
        let bridges = self.get_bridges(piers).await;
        self.build_bridges(bridges).await;
    }

    async fn get_piers(&self) -> Vec<[Pier; 3]> {
        self.cx
            .with_world(|world| get_piers(&world, &self.parameters.deep_sea_level))
            .await
    }

    async fn get_bridges(&self, piers: Vec<[Pier; 3]>) -> Vec<Bridge> {
        piers
            .into_iter()
            .flat_map(|piers| {
                Bridge {
                    piers: piers.to_vec(),
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

fn get_piers(world: &World, deep_sea_level: &f32) -> Vec<[Pier; 3]> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            for offset in [v2(1, 0), v2(0, 1)].iter() {
                let from = v2(x, y);

                if let Some(to) = world.offset(&from, *offset) {
                    if let Some(pier) = is_pier(&world, &from, &to, deep_sea_level) {
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
    deep_sea_level: &f32,
) -> Option<[Pier; 3]> {
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

    if to_elevation > *deep_sea_level {
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
            elevation: sea_level,
            platform: false,
        },
        Pier {
            position: *to,
            elevation: sea_level,
            platform: false,
        },
    ])
}
