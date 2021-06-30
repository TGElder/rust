use std::collections::HashSet;

use commons::edge::Edge;

use crate::route::{Route, RouteKey, Routes, RoutesExt};
use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::traits::{WithEdgeTraffic, WithRoutes};

pub struct RouteSummary {
    pub traffic: usize,
    pub first_visit: u128,
}

impl From<&Route> for RouteSummary {
    fn from(route: &Route) -> Self {
        RouteSummary {
            traffic: route.traffic,
            first_visit: route.start_micros + route.duration.as_micros(),
        }
    }
}

impl<T, D> EdgeBuildSimulation<T, D>
where
    T: WithRoutes + WithEdgeTraffic,
{
    pub async fn get_route_summaries(&self, edge: &Edge) -> Vec<RouteSummary> {
        let route_keys = self.get_edge_traffic(edge).await;
        if route_keys.is_empty() {
            return vec![];
        }

        self.cx
            .with_routes(|routes| get_route_summaries(routes, route_keys))
            .await
    }

    async fn get_edge_traffic(&self, edge: &Edge) -> HashSet<RouteKey> {
        self.cx
            .with_edge_traffic(|edge_traffic| edge_traffic.get(edge).cloned().unwrap_or_default())
            .await
    }

    pub fn get_when(&self, mut routes: Vec<RouteSummary>, threshold: usize) -> u128 {
        routes.sort_by_key(|route| route.first_visit);
        let mut traffic_cum = 0;
        for route in routes {
            traffic_cum += route.traffic;
            if traffic_cum >= threshold {
                return route.first_visit;
            }
        }
        panic!(
            "Total traffic {} does not exceed threshold {}",
            traffic_cum, threshold
        );
    }
}

fn get_route_summaries(routes: &Routes, route_keys: HashSet<RouteKey>) -> Vec<RouteSummary> {
    route_keys
        .into_iter()
        .flat_map(|route_key| routes.get_route(&route_key))
        .map(|route| route.into())
        .collect()
}
