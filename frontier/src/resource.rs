use std::collections::HashSet;

use commons::index2d::Vec2D;
use commons::rand::Rng;
use serde::{Deserialize, Serialize};

use crate::world::WorldObject;

pub const RESOURCES: [Resource; 18] = [
    Resource::Bananas,
    Resource::Bison,
    Resource::Coal,
    Resource::Crabs,
    Resource::Crops,
    Resource::Deer,
    Resource::Fur,
    Resource::Gems,
    Resource::Gold,
    Resource::Iron,
    Resource::Ivory,
    Resource::Pasture,
    Resource::Shelter,
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
    Bison,
    Coal,
    Crabs,
    Crops,
    Deer,
    Fur,
    Gems,
    Gold,
    Iron,
    Ivory,
    Pasture,
    Shelter,
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
            Resource::Bison => "bison",
            Resource::Coal => "coal",
            Resource::Crabs => "crabs",
            Resource::Crops => "crops",
            Resource::Deer => "deer",
            Resource::Fur => "fur",
            Resource::Gems => "gems",
            Resource::Gold => "gold",
            Resource::Iron => "iron",
            Resource::Ivory => "ivory",
            Resource::Pasture => "pasture",
            Resource::Shelter => "shelter",
            Resource::Spice => "spice",
            Resource::Stone => "stone",
            Resource::Truffles => "truffles",
            Resource::Whales => "whales",
            Resource::Wood => "wood",
        }
    }
}

pub type Resources = Vec2D<HashSet<Resource>>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MineRule {
    pub resource: Resource,
    pub mine: Mine,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum Mine {
    None,
    Crop,
    Pasture,
    House,
}

impl Mine {
    pub fn matches(&self, world_object: &WorldObject) -> bool {
        matches!(
            (self, world_object),
            (Mine::None, WorldObject::None)
                | (Mine::None, WorldObject::Vegetation { .. })
                | (Mine::Crop, WorldObject::Crop { .. })
                | (Mine::Pasture, WorldObject::Pasture)
                | (Mine::House, WorldObject::House { .. })
        )
    }

    pub fn get_world_object<R: Rng>(&self, rng: &mut R) -> WorldObject {
        match self {
            Mine::None => WorldObject::None,
            Mine::Crop => WorldObject::Crop { rotated: rng.gen() },
            Mine::Pasture => WorldObject::Pasture,
            Mine::House => WorldObject::House { rotated: rng.gen() },
        }
    }
}
