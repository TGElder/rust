use super::*;
use crate::settlement::*;

mod demand;
mod resource_routes_sim;
mod resource_routes_targets;

pub use resource_routes_sim::ResourceRouteSim;
pub use resource_routes_targets::ResourceRouteTargets;

const RESOURCES: [Resource; 3] = [Resource::Farmland, Resource::Gems, Resource::Oranges];
