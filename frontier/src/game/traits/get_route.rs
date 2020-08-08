use crate::game::traits::Routes;
use crate::route::{Route, RouteKey};
use std::collections::HashMap;

pub trait GetRoute {
    fn get_route(&self, route_key: &RouteKey) -> Option<&Route>;
}

impl<T> GetRoute for T
where
    T: Routes,
{
    fn get_route(&self, route_key: &RouteKey) -> Option<&Route> {
        self.routes()
            .get(&route_key.into())
            .and_then(|route_set| route_set.get(route_key))
    }
}

impl GetRoute for HashMap<RouteKey, Route> {
    fn get_route(&self, route_key: &RouteKey) -> Option<&Route> {
        self.get(route_key)
    }
}
