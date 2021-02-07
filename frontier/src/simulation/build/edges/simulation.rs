use commons::edge::Edge;

use crate::simulation::build::edges::processors::{BuildRoad, RemoveRoad};
use crate::traits::{
    InsertBuildInstruction, IsRoad, PlanRoad, RemoveBuildInstruction,
    RemoveRoad as RemoveRoadTrait, RoadPlanned, SendRoutes, SendWorld, WithEdgeTraffic,
};
use crate::travel_duration::TravelDuration;

use std::collections::HashSet;

pub struct EdgeBuildSimulation<T, D> {
    pub build_road: BuildRoad<T, D>,
    pub remove_road: RemoveRoad<T>,
}

impl<T, D> EdgeBuildSimulation<T, D>
where
    T: InsertBuildInstruction
        + IsRoad
        + PlanRoad
        + RemoveBuildInstruction
        + RemoveRoadTrait
        + RoadPlanned
        + SendRoutes
        + SendWorld
        + WithEdgeTraffic
        + Send
        + Sync,
    D: TravelDuration + 'static,
{
    pub async fn refresh_edges(&mut self, edges: HashSet<Edge>) {
        join!(
            self.build_road.refresh_edges(&edges),
            self.remove_road.refresh_edges(&edges),
        );
    }
}
