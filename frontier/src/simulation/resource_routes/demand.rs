use super::*;

const HOMELAND_DEMAND: [Resource; 7] = [
    Resource::Bananas,
    Resource::Deer,
    Resource::Fur,
    Resource::Gems,
    Resource::Gold,
    Resource::Ivory,
    Resource::Spice,
];

#[derive(Debug)]
pub struct Demand {
    pub position: V2<usize>,
    pub resource: Resource,
    pub sources: usize,
    pub quantity: usize,
}

pub fn get_demands(settlement: &Settlement) -> Vec<Demand> {
    match settlement.class {
        SettlementClass::Town => RESOURCES
            .iter()
            .map(|resource| get_demand(settlement, *resource))
            .collect(),
        SettlementClass::Homeland => RESOURCES
            .iter()
            .filter(|resource| HOMELAND_DEMAND.contains(resource))
            .map(|resource| get_demand(settlement, *resource))
            .collect(),
    }
}

fn get_demand(settlement: &Settlement, resource: Resource) -> Demand {
    Demand {
        position: settlement.position,
        resource,
        sources: get_sources(settlement.population, resource),
        quantity: get_quantity(settlement.population, resource),
    }
}

fn get_quantity(population: usize, resource: Resource) -> usize {
    match resource {
        Resource::Bananas => population / 32,
        Resource::Coal => population / 8,
        Resource::Deer => population / 32,
        Resource::Fur => population / 32,
        Resource::Gems => population / 128,
        Resource::Gold => population / 128,
        Resource::Iron => population / 8,
        Resource::Ivory => population / 128,
        Resource::Spice => population / 32,
        Resource::Stone => population / 4,
        _ => 1,
    }
}

fn get_sources(population: usize, resource: Resource) -> usize {
    match resource {
        Resource::Farmland => population,
        Resource::Wood => population / 2,
        _ => 1,
    }
}
