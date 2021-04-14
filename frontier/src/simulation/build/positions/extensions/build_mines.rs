use std::collections::{HashMap, HashSet};

use commons::V2;
use commons::grid::Grid;

use crate::resource::{Mine, Resource};
use crate::route::RouteKey;
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::traffic::Traffic;
use crate::traits::{InsertBuildInstruction, Micros, WithTraffic, WithWorld};
use crate::traits::has::HasParameters;
use crate::world::WorldObject;

impl <T> PositionBuildSimulation<T> 
    where T: HasParameters + InsertBuildInstruction + Micros + WithTraffic + WithWorld
{
    fn build_mines(&self) {
        
        
    }


    async fn plan_mines(&self, positions: HashSet<V2<usize>>) -> HashMap<V2<usize>, Option<WorldObject>> {
        let mines = &self.cx.parameters().mines;

        self.cx.with_traffic(|traffic| {
            positions.into_iter().map(|position| (position, plan_mine(traffic, &position, mines))).collect()
        }).await
    }

    async fn get_changes(&self, plans: HashMap<V2<usize>, Option<WorldObject>>) -> HashMap<V2<usize>, Option<WorldObject>> {
        self.cx.with_world(|world| {
            plans.into_iter().filter(|(position, plan)| is_change(plan, world.get_cell_unsafe(position).object)).collect()
        }).await
    }

    async fn apply_changes(&self, changes: HashMap<V2<usize>, Option<WorldObject>>) {
        
    }


}


fn plan_mine(traffic: &Traffic, position: &V2<usize>, mines: &[Mine]) -> Option<WorldObject> {
    let traffic = traffic.get_cell_unsafe(position);
    for mine in mines {
        if traffic.iter().any(|RouteKey{resource, ..}| *resource == mine.resource) {
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