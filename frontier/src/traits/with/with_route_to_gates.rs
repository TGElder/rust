use std::collections::{HashMap, HashSet};

use commons::async_trait::async_trait;
use commons::V2;

use crate::route::RouteKey;

#[async_trait]
pub trait WithRouteToGates {
    async fn with_route_to_gates<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send;

    async fn mut_route_to_gates<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send;
}
