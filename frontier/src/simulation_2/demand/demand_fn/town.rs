use super::*;

use crate::resource::RESOURCES;

pub fn town_demand_fn(settlement: &Settlement) -> Vec<Demand> {
    if settlement.class != SettlementClass::Town {
        return vec![];
    }
    RESOURCES
        .iter()
        .map(move |resource| get_demand(&settlement, *resource))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_no_demand_if_settlement_not_town() {
        let settlement = Settlement {
            class: SettlementClass::Homeland,
            ..Settlement::default()
        };
        assert!(town_demand_fn(&settlement).is_empty());
    }
}
