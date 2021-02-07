use super::*;

const HOMELAND_RESOURCES: [Resource; 10] = [
    Resource::Bananas,
    Resource::Bison,
    Resource::Deer,
    Resource::Fur,
    Resource::Gems,
    Resource::Gold,
    Resource::Ivory,
    Resource::Spice,
    Resource::Truffles,
    Resource::Whales,
];

pub fn homeland_demand_fn(settlement: &Settlement) -> Vec<Demand> {
    if settlement.class != SettlementClass::Homeland {
        return vec![];
    }
    HOMELAND_RESOURCES
        .iter()
        .map(move |resource| get_demand(&settlement, *resource))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_no_demand_if_settlement_not_homeland() {
        let settlement = Settlement {
            class: SettlementClass::Town,
            ..Settlement::default()
        };
        assert!(homeland_demand_fn(&settlement).is_empty());
    }
}
