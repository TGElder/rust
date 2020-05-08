use serde::{Deserialize, Serialize};

pub const RESOURCES: [Resource; 15] = [
    Resource::Bananas,
    Resource::Coal,
    Resource::Crabs,
    Resource::Deer,
    Resource::Farmland,
    Resource::Fur,
    Resource::Gems,
    Resource::Gold,
    Resource::Iron,
    Resource::Ivory,
    Resource::Spice,
    Resource::Stone,
    Resource::Truffles,
    Resource::Whales,
    Resource::Wood,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Resource {
    None,
    Bananas,
    Coal,
    Crabs,
    Deer,
    Farmland,
    Fur,
    Gems,
    Gold,
    Iron,
    Ivory,
    Spice,
    Stone,
    Truffles,
    Whales,
    Wood,
}

impl Resource {
    pub fn name(self) -> &'static str {
        match self {
            Resource::None => "none",
            Resource::Bananas => "bananas",
            Resource::Coal => "coal",
            Resource::Crabs => "crabs",
            Resource::Deer => "deer",
            Resource::Farmland => "farmland",
            Resource::Fur => "fur",
            Resource::Gems => "gems",
            Resource::Gold => "gold",
            Resource::Iron => "iron",
            Resource::Ivory => "ivory",
            Resource::Spice => "spice",
            Resource::Stone => "stone",
            Resource::Truffles => "truffles",
            Resource::Whales => "whales",
            Resource::Wood => "wood",
        }
    }
}
