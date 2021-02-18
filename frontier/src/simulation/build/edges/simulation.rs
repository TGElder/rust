use commons::edge::Edge;

use crate::traits::has::HasParameters;
use crate::traits::{
    InsertBuildInstruction, IsRoad, PlanRoad, RemoveBuildInstruction,
    RemoveRoad as RemoveRoadTrait, RoadPlanned, WithEdgeTraffic, WithRoutes, WithWorld,
};
use crate::travel_duration::TravelDuration;

use std::collections::HashSet;
use std::sync::Arc;

pub struct EdgeBuildSimulation<T, D> {
    pub(super) tx: T,
    pub(super) travel_duration: Arc<D>,
}

impl<T, D> EdgeBuildSimulation<T, D> {
    pub fn new(tx: T, travel_duration: Arc<D>) -> EdgeBuildSimulation<T, D> {
        EdgeBuildSimulation {
            tx,
            travel_duration,
        }
    }
}

impl<T, D> EdgeBuildSimulation<T, D>
where
    T: HasParameters
        + InsertBuildInstruction
        + IsRoad
        + PlanRoad
        + RemoveBuildInstruction
        + RemoveRoadTrait
        + RoadPlanned
        + WithRoutes
        + WithEdgeTraffic
        + WithWorld
        + Send
        + Sync,
    D: TravelDuration + 'static,
{
    pub async fn refresh_edges(&mut self, edges: HashSet<Edge>) {
        join!(self.build_road(&edges), self.remove_road(&edges),);
    }
}
