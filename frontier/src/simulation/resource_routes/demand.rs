use super::*;

const OLD_WORLD_DEMAND: [Resource; 7] = [
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
        SettlementClass::OldWorld => RESOURCES
            .iter()
            .filter(|resource| OLD_WORLD_DEMAND.contains(resource))
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
        Resource::Farmland => 1,
        Resource::Fur => population / 32,
        Resource::Gems => population / 128,
        Resource::Gold => population / 128,
        Resource::Iron => population / 8,
        Resource::Ivory => population / 128,
        Resource::Spice => population / 32,
        Resource::Stone => population / 4,
        Resource::Wood => population / 2,
        _ => 0,
    }
}

fn get_sources(population: usize, resource: Resource) -> usize {
    match resource {
        Resource::Farmland => population,
        _ => 1,
    }
}
