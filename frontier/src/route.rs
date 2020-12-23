use crate::resource::Resource;
use commons::V2;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub path: Vec<V2<usize>>,
    pub start_micros: u128,
    pub duration: Duration,
    pub traffic: usize,
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteKey {
    pub settlement: V2<usize>,
    pub resource: Resource,
    pub destination: V2<usize>,
}

impl Display for RouteKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{},{}-{:?}-{},{}",
            self.settlement.x,
            self.settlement.y,
            self.resource.name(),
            self.destination.x,
            self.destination.y
        )
    }
}

pub type RouteSet = HashMap<RouteKey, Route>;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteSetKey {
    pub settlement: V2<usize>,
    pub resource: Resource,
}

impl<T> From<T> for RouteSetKey
where
    T: Borrow<RouteKey>,
{
    fn from(route_key: T) -> RouteSetKey {
        RouteSetKey {
            settlement: route_key.borrow().settlement,
            resource: route_key.borrow().resource,
        }
    }
}

pub type Routes = HashMap<RouteSetKey, RouteSet>;

pub trait RoutesExt {
    fn get_route(&self, key: &RouteKey) -> Option<&Route>;
    fn insert_route(&mut self, key: RouteKey, route: Route);
}

impl RoutesExt for Routes {
    fn get_route(&self, key: &RouteKey) -> Option<&Route> {
        self.get(&key.into())
            .and_then(|route_set| route_set.get(key))
    }

    fn insert_route(&mut self, key: RouteKey, route: Route) {
        self.entry(key.into())
            .or_insert_with(HashMap::new)
            .insert(key, route);
    }
}
