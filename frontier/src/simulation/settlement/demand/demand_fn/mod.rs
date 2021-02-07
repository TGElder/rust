use super::*;

mod homeland;
mod town;

pub use homeland::*;
pub use town::*;

use crate::resource::Resource;
use crate::settlement::{Settlement, SettlementClass};

fn get_demand(settlement: &Settlement, resource: Resource) -> Demand {
    Demand {
        position: settlement.position,
        resource,
        sources: get_sources(settlement.current_population, resource),
        quantity: get_quantity(settlement.current_population, resource),
    }
}

fn get_sources(population: f64, resource: Resource) -> usize {
    let sources = match resource {
        Resource::Crops => population / 2.0,
        Resource::Pasture => population / 2.0,
        Resource::Stone => population / 8.0,
        Resource::Wood => population / 4.0,
        _ => 1.0,
    };
    sources.round() as usize
}

fn get_quantity(population: f64, resource: Resource) -> usize {
    let sources = match resource {
        Resource::Bananas => population / 32.0,
        Resource::Bison => population / 32.0,
        Resource::Crabs => population / 32.0,
        Resource::Coal => population / 16.0,
        Resource::Deer => population / 32.0,
        Resource::Fur => population / 32.0,
        Resource::Gems => population / 64.0,
        Resource::Gold => population / 64.0,
        Resource::Iron => population / 16.0,
        Resource::Ivory => population / 64.0,
        Resource::Spice => population / 32.0,
        Resource::Truffles => population / 64.0,
        Resource::Whales => population / 64.0,
        _ => 1.0,
    };
    sources.round() as usize
}
