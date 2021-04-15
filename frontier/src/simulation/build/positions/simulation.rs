use commons::V2;

use crate::traits::has::HasParameters;
use crate::traits::{
    AnyoneControls, GetBuildInstruction, GetSettlement, InsertBuildInstruction, Micros,
    RandomTownName, RefreshTargets, RemoveBuildInstruction, RemoveWorldObject, WithRouteToPorts,
    WithRoutes, WithTraffic, WithWorld,
};

use std::collections::HashSet;

pub struct PositionBuildSimulation<T> {
    pub(super) cx: T,
}

impl<T> PositionBuildSimulation<T> {
    pub fn new(cx: T) -> PositionBuildSimulation<T> {
        PositionBuildSimulation { cx }
    }
}

impl<T> PositionBuildSimulation<T>
where
    T: AnyoneControls
        + GetBuildInstruction
        + GetSettlement
        + HasParameters
        + InsertBuildInstruction
        + Micros
        + RandomTownName
        + RefreshTargets
        + RemoveBuildInstruction
        + RemoveWorldObject
        + WithRoutes
        + WithRouteToPorts
        + WithTraffic
        + WithWorld,
{
    pub async fn refresh_positions(&mut self, positions: HashSet<V2<usize>>) {
        join!(
            self.build_town(positions.clone()),
            self.build_mines(positions),
        );
    }
}
