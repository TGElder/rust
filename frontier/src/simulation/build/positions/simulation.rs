use commons::V2;

use crate::simulation::build::positions::processors::{BuildCrops, BuildTown, RemoveCrops};
use crate::traits::{
    AnyoneControls, GetBuildInstruction, GetSettlement, InsertBuildInstruction, RandomTownName,
    RemoveBuildInstruction, RemoveWorldObject, SendRoutes, SendWorld, WithRouteToPorts,
    WithTraffic,
};

use std::collections::HashSet;

pub struct PositionBuildSimulation<T> {
    pub build_crops: BuildCrops<T>,
    pub build_town: BuildTown<T>,
    pub remove_crops: RemoveCrops<T>,
}

impl<T> PositionBuildSimulation<T>
where
    T: AnyoneControls
        + GetBuildInstruction
        + GetSettlement
        + InsertBuildInstruction
        + RandomTownName
        + RemoveBuildInstruction
        + RemoveWorldObject
        + SendRoutes
        + SendWorld
        + WithRouteToPorts
        + WithTraffic,
{
    pub async fn refresh_positions(&mut self, positions: HashSet<V2<usize>>) {
        join!(
            self.build_crops.refresh_positions(positions.clone()),
            self.build_town.refresh_positions(positions.clone()),
            self.remove_crops.refresh_positions(positions),
        );
    }
}
