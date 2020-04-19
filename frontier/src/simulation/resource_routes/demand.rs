use super::*;

#[derive(Debug)]
pub struct Demand {
    pub position: V2<usize>,
    pub resource: Resource,
    pub quantity: usize,
    pub sources: usize,
}

pub fn get_demand(settlement: &Settlement) -> Vec<Demand> {
    let position = settlement.position;
    let population = settlement.population;
    match settlement.class {
        SettlementClass::Town => vec![
            Demand {
                position,
                resource: Resource::Gems,
                quantity: 1,
                sources: population / 32,
            },
            Demand {
                position,
                resource: Resource::Oranges,
                quantity: 1,
                sources: population / 16,
            },
            Demand {
                position,
                resource: Resource::Farmland,
                quantity: 1,
                sources: population,
            },
        ],
        SettlementClass::OldWorld => vec![
            Demand {
                position,
                resource: Resource::Gems,
                quantity: 1,
                sources: population / 32,
            },
            Demand {
                position,
                resource: Resource::Oranges,
                quantity: 1,
                sources: population / 16,
            },
        ],
    }
}
