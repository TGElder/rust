use serde::{Deserialize, Serialize};

pub const RESOURCES: [Resource; 11] = [
    Resource::Bananas,
    Resource::Coal,
    Resource::Deer,
    Resource::Farmland,
    Resource::Fur,
    Resource::Gems,
    Resource::Gold,
    Resource::Iron,
    Resource::Ivory,
    Resource::Spice,
    Resource::Stone,
];

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub enum Resource {
    None,
    Bananas,
    Coal,
    Deer,
    Farmland,
    Fur,
    Gems,
    Gold,
    Iron,
    Ivory,
    Spice,
    Stone,
}

impl Resource {
    pub fn name(self) -> &'static str {
        match self {
            Resource::None => "none",
            Resource::Bananas => "bananas",
            Resource::Coal => "coal",
            Resource::Deer => "deer",
            Resource::Farmland => "farmland",
            Resource::Fur => "fur",
            Resource::Gems => "gems",
            Resource::Gold => "gold",
            Resource::Iron => "iron",
            Resource::Ivory => "ivory",
            Resource::Spice => "spice",
            Resource::Stone => "stone",
        }
    }
}
