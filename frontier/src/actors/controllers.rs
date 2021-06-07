use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;

use commons::async_std::task::sleep;
use commons::async_trait::async_trait;
use commons::grid::get_corners_in_bounds;
use commons::grid::Grid;
use commons::index2d::Vec2D;
use commons::process::Step;
use commons::v2;
use commons::M;
use commons::V2;

use crate::settlement::Settlement;
use crate::settlement::SettlementClass::Homeland;
use crate::territory::Controllers;
use crate::traits::has::HasParameters;
use crate::traits::DrawWorld;
use crate::traits::WithWorld;
use crate::traits::{PathfinderForRoutes, Settlements, WithControllers, WithPathfinder};
use crate::world::World;

pub struct ControllersActor<T> {
    cx: T,
    parameters: ControllersActorParameters,
}

pub struct ControllersActorParameters {
    pub refresh_interval: Duration,
}

impl Default for ControllersActorParameters {
    fn default() -> ControllersActorParameters {
        ControllersActorParameters {
            refresh_interval: Duration::from_secs(10),
        }
    }
}

impl<T> ControllersActor<T>
where
    T: DrawWorld + HasParameters + PathfinderForRoutes + Settlements + WithControllers + WithWorld,
{
    pub fn new(t: T, parameters: ControllersActorParameters) -> ControllersActor<T> {
        ControllersActor { cx: t, parameters }
    }

    async fn update_controllers(&self) {
        let new_controllers = self.get_controllers().await;

        let changes = self.get_changes(&new_controllers).await;

        self.cx
            .mut_controllers(|controllers| *controllers = new_controllers)
            .await;

        self.cx.draw_world_tiles(changes).await;
    }

    async fn get_controllers(&self) -> Controllers {
        let settlements = self.cx.settlements().await;

        let (homelands, towns): (Vec<Settlement>, Vec<Settlement>) = settlements
            .into_iter()
            .partition(|settlement| settlement.class == Homeland);

        let nation_to_origin = homelands
            .into_iter()
            .map(|homeland| (homeland.nation, homeland.position))
            .collect::<HashMap<_, _>>();

        let width = self.cx.parameters().width;

        let mut origin_to_positions = nation_to_origin
            .values()
            .map(|origin| (*origin, vec![]))
            .collect::<HashMap<_, _>>();
        for town in towns {
            let origin = unwrap_or!(nation_to_origin.get(&town.nation), continue);
            let positions = origin_to_positions.get_mut(origin).unwrap();
            positions.append(&mut get_corners_in_bounds(&town.position, &width, &width));
        }

        let closest_origins = self
            .cx
            .routes_pathfinder()
            .with_pathfinder(|pathfinder| pathfinder.closest_origins(&origin_to_positions))
            .await;

        self.get_controllers_from_closest_origins(closest_origins)
            .await
    }

    async fn get_controllers_from_closest_origins(
        &self,
        closest_origins: Vec2D<HashSet<V2<usize>>>,
    ) -> Controllers {
        self.cx
            .with_world(|world| {
                M::from_fn(closest_origins.width(), closest_origins.height(), |x, y| {
                    get_controller(&world, &closest_origins, &v2(x, y))
                })
            })
            .await
    }

    async fn get_changes(&self, new_controllers: &Controllers) -> HashSet<V2<usize>> {
        self.cx
            .with_controllers(|controllers| {
                let mut out = hashset! {};
                for x in 0..controllers.width() {
                    for y in 0..controllers.height() {
                        let position = v2(x, y);
                        if controllers.get_cell_unsafe(&position)
                            != new_controllers.get_cell_unsafe(&position)
                        {
                            out.insert(position);
                        }
                    }
                }
                out
            })
            .await
    }
}

#[derive(Default)]
struct ControlCounts {
    from_land: usize,
    from_water: usize,
}

fn get_controller(
    world: &World,
    closest_origins: &Vec2D<HashSet<V2<usize>>>,
    tile: &V2<usize>,
) -> Option<V2<usize>> {
    let mut candidates: HashMap<V2<usize>, ControlCounts> = hashmap! {};
    for corner in world.get_corners_in_bounds(tile) {
        for controller in closest_origins.get_cell_unsafe(&corner) {
            let mut counts = candidates.entry(*controller).or_default();
            if world.is_sea(&corner) || world.get_cell_unsafe(&corner).river.here() {
                counts.from_water += 1;
            } else {
                counts.from_land += 1;
            }
        }
    }

    candidates
        .into_iter()
        .max_by_key(|(position, counts)| {
            (counts.from_land, counts.from_water, position.x, position.y)
        })
        .map(|(position, _)| position)
}

#[async_trait]
impl<T> Step for ControllersActor<T>
where
    T: DrawWorld
        + HasParameters
        + PathfinderForRoutes
        + Settlements
        + WithControllers
        + WithWorld
        + Send
        + Sync,
{
    async fn step(&mut self) {
        self.update_controllers().await;

        sleep(self.parameters.refresh_interval).await;
    }
}
