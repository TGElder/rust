use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::{Rotation, Vehicle};
use crate::bridges::{Bridge, BridgeType, Pier};
use crate::traits::{WithBridges, WithWorld};
use crate::world::World;

pub struct SeaPiers<T> {
    cx: T,
    parameters: SeaPierParameters,
}

pub struct SeaPierParameters {
    pub deep_sea_level: f32,
    pub max_landing_zone_gradient: f32,
    pub max_gradient: f32,
}

impl<T> SeaPiers<T>
where
    T: WithBridges + WithWorld + Sync,
{
    pub fn new(cx: T, parameters: SeaPierParameters) -> SeaPiers<T> {
        SeaPiers { cx, parameters }
    }

    pub async fn new_game(&self) {
        let piers = self.get_piers().await;
        let bridges = self.get_bridges(piers).await;
        self.build_bridges(bridges).await;
    }

    async fn get_piers(&self) -> Vec<[Pier; 4]> {
        self.cx
            .with_world(|world| get_piers(&world, &self.parameters))
            .await
    }

    async fn get_bridges(&self, piers: Vec<[Pier; 4]>) -> Vec<Bridge> {
        piers
            .into_iter()
            .flat_map(|piers| {
                Bridge {
                    piers: piers.to_vec(),

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

fn get_piers(world: &World, parameters: &SeaPierParameters) -> Vec<[Pier; 4]> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            for offset in [v2(1, 0), v2(0, 1)].iter() {
                let from = v2(x, y);

                if let Some(to) = world.offset(&from, *offset) {
                    if let Some(pier) = is_pier(&world, &from, &to, parameters) {
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
    parameters: &SeaPierParameters,
) -> Option<[Pier; 4]> {
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

    if to_elevation > parameters.deep_sea_level {
        return None;
    }

    if world.get_rise(from, to)?.abs() > parameters.max_gradient {
        return None;
    }

    if !has_launching_zone(world, from, &parameters.max_landing_zone_gradient) {
        return None;
    }

    let rotation = Rotation::from_positions(from, to).ok()?;
    Some([
        Pier {
            position: *from,
            elevation: from_elevation,
            platform: true,
            rotation,
            vehicle: Vehicle::None,
        },
        Pier {
            position: *to,
            elevation: sea_level,
            platform: false,
            rotation,
            vehicle: Vehicle::None,
        },
        Pier {
            position: *to,
            elevation: sea_level,
            platform: false,
            rotation,
            vehicle: Vehicle::Boat,
        },
        Pier {
            position: *to,
            elevation: sea_level,
            platform: false,
            rotation,
            vehicle: Vehicle::Boat,
        },
    ])
}

fn has_launching_zone(
    world: &World,
    position: &V2<usize>,
    max_landing_zone_gradient: &f32,
) -> bool {
    world
        .get_adjacent_tiles_in_bounds(position)
        .iter()
        .any(|tile| {
            !world.is_sea(tile) && world.get_max_abs_rise(tile) <= *max_landing_zone_gradient
        })
}
