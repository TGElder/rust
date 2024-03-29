use commons::edge::Edge;

use crate::traits::has::HasParameters;
use crate::traits::{
    GetBuildInstruction, InsertBuildInstruction, IsRoad, PlanRoad, RemoveBuildInstruction,
    RemoveRoad as RemoveRoadTrait, RoadPlanned, WithBridges, WithEdgeTraffic, WithRoutes,
    WithWorld,
};
use crate::travel_duration::TravelDuration;

use std::collections::HashSet;
use std::sync::Arc;

pub struct EdgeBuildSimulation<T, D> {
    pub(super) cx: T,
    pub(super) travel_duration: Arc<D>,
}

impl<T, D> EdgeBuildSimulation<T, D> {
    pub fn new(cx: T, travel_duration: Arc<D>) -> EdgeBuildSimulation<T, D> {
        EdgeBuildSimulation {
            cx,
            travel_duration,
        }
    }
}

impl<T, D> EdgeBuildSimulation<T, D>
where
    T: GetBuildInstruction
        + HasParameters
        + InsertBuildInstruction
        + IsRoad
        + PlanRoad
        + RemoveBuildInstruction
        + RemoveRoadTrait
        + RoadPlanned
        + WithBridges
        + WithRoutes
        + WithEdgeTraffic
        + WithWorld
        + Send
        + Sync,
    D: TravelDuration,
{
    pub async fn refresh_edges(&mut self, edges: HashSet<Edge>) {
        join!(
            self.build_road(&edges),
            self.build_bridge(&edges),
            self.remove_road(&edges),
        );
    }
}
