use super::*;

mod resource_routes_sim;
mod resource_routes_targets;

pub use resource_routes_sim::ResourceRouteSim;
pub use resource_routes_targets::ResourceRouteTargets;

const RESOURCES: [Resource; 2] = [Resource::Gems, Resource::Oranges];
