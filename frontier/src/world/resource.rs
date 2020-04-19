use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub enum Resource {
    None,
    Farmland,
    Gems,
    Oranges,
}

impl Resource {
    pub fn name(self) -> &'static str {
        match self {
            Resource::None => "none",
            Resource::Farmland => "farmland",
            Resource::Gems => "gems",
            Resource::Oranges => "oranges",
        }
    }
}
