use crate::route::RouteKey;
use commons::index2d::Vec2D;
use std::collections::HashSet;

pub type Traffic = Vec2D<HashSet<RouteKey>>;
