use crate::route::RouteKey;
use commons::edge::Edge;
use commons::index2d::Vec2D;
use std::collections::{HashMap, HashSet};

pub type Traffic = Vec2D<HashSet<RouteKey>>;
pub type EdgeTraffic = HashMap<Edge, HashSet<RouteKey>>;
