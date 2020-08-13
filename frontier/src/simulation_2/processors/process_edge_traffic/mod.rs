use super::*;

mod get_edge_traffic;
#[allow(clippy::module_inception)]
mod process_edge_traffic;
mod try_build_road;
mod update_edge_traffic;

use get_edge_traffic::get_edge_traffic;
pub use process_edge_traffic::ProcessEdgeTraffic;
use try_build_road::try_build_road;
use update_edge_traffic::update_all_edge_traffic_and_get_changes;
