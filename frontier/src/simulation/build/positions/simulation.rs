use commons::rand::prelude::SmallRng;
use commons::rand::SeedableRng;
use commons::V2;

use crate::traits::has::HasParameters;
use crate::traits::{
    AnyoneControls, GetBuildInstruction, GetSettlement, InsertBuildInstruction, RandomTownName,
    RemoveBuildInstruction, RemoveWorldObject, SendRoutes, WithRouteToPorts, WithTraffic,
    WithWorld,
};

use std::collections::HashSet;

pub struct PositionBuildSimulation<T> {
    pub(super) tx: T,
    pub(super) rng: SmallRng,
}

impl<T> PositionBuildSimulation<T> {
    pub fn new(tx: T, seed: u64) -> PositionBuildSimulation<T> {
        PositionBuildSimulation {
            tx,
            rng: SeedableRng::seed_from_u64(seed),
        }
    }
}

impl<T> PositionBuildSimulation<T>
where
    T: AnyoneControls
        + GetBuildInstruction
        + GetSettlement
        + HasParameters
        + InsertBuildInstruction
        + RandomTownName
        + RemoveBuildInstruction
        + RemoveWorldObject
        + SendRoutes
        + WithRouteToPorts
        + WithTraffic
        + WithWorld,
{
    pub async fn refresh_positions(&mut self, positions: HashSet<V2<usize>>) {
        self.build_crops(positions.clone()).await;
        join!(
            self.build_town(positions.clone()),
            self.remove_crops(positions),
        );
    }
}
