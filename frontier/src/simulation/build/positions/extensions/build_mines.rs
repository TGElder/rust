use std::collections::{HashMap, HashSet};

use commons::grid::Grid;
use commons::log::debug;
use commons::V2;

use crate::build::{Build, BuildInstruction};
use crate::resource::Mine;
use crate::route::RouteKey;
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::traffic::Traffic;
use crate::traits::has::HasParameters;
use crate::traits::{InsertBuildInstruction, Micros, WithTraffic, WithWorld};
use crate::world::WorldObject;

impl<T> PositionBuildSimulation<T>
where
    T: HasParameters + InsertBuildInstruction + Micros + WithTraffic + WithWorld,
{
    pub async fn build_mines(&self, positions: HashSet<V2<usize>>) {
        let plans = self.plan_mines(positions).await;
        let changes = self.get_changes(plans).await;
        self.apply_changes(changes).await;
    }

    async fn plan_mines(
        &self,
        positions: HashSet<V2<usize>>,
    ) -> HashMap<V2<usize>, Option<WorldObject>> {
        let mines = &self.cx.parameters().mines;

        self.cx
            .with_traffic(|traffic| {
                positions
                    .into_iter()
                    .map(|position| (position, plan_mine(traffic, &position, mines)))
                    .collect()
            })
            .await
    }

    async fn get_changes(
        &self,
        plans: HashMap<V2<usize>, Option<WorldObject>>,
    ) -> HashMap<V2<usize>, Option<WorldObject>> {
        self.cx
            .with_world(|world| {
                plans
                    .into_iter()
                    .filter(|(position, plan)| {
                        let is_change = is_change(plan, world.get_cell_unsafe(position).object);
                        if is_change {
                            debug!(
                                "Expected {:?} found {:?}",
                                plan,
                                world.get_cell_unsafe(position).object
                            );
                        }
                        is_change
                    })
                    .collect()
            })
            .await
    }

    async fn apply_changes(&self, changes: HashMap<V2<usize>, Option<WorldObject>>) {
        let when = self.cx.micros().await;
        for (position, change) in changes {
            self.cx
                .insert_build_instruction(BuildInstruction {
                    what: Build::Object {
                        position,
                        object: change.unwrap_or(WorldObject::None),
                    },
                    when,
                })
                .await
        }
    }
}

fn plan_mine(traffic: &Traffic, position: &V2<usize>, mines: &[Mine]) -> Option<WorldObject> {
    let traffic = traffic.get_cell_unsafe(position);
    for mine in mines {
        if traffic
            .iter()
            .filter(|RouteKey { destination, .. }| position == destination)
            .any(|RouteKey { resource, .. }| *resource == mine.resource)
        {
            return Some(mine.mine);
        }
    }
    None
}

fn is_change(plan: &Option<WorldObject>, actual: WorldObject) -> bool {
    if let Some(plan) = plan {
        *plan != actual
    } else {
        matches!(actual, WorldObject::Crop{..})
    }
}
