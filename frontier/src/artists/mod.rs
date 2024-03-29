mod avatar_artist;
mod bridge_artist;
mod crop_artist;
mod house_artist;
mod resource_artist;
mod sprite_sheet;
mod vegetation_artist;
mod world_artist;

use crate::world::*;
use commons::edge::*;
use commons::*;
use isometric::Command;
use std::default::Default;

pub use avatar_artist::{AvatarArtist, AvatarArtistParameters};
pub use bridge_artist::{BridgeArtist, BridgeArtistParameters};
pub use house_artist::{HouseArtist, HouseArtistParameters};
pub use resource_artist::{ResourceArtist, ResourceArtistParameters};
pub use world_artist::{Slab, WorldArtist, WorldArtistParameters, WorldColoring};
