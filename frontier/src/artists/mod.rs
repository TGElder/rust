mod avatar_artist;
mod crop_artist;
mod fast_avatar_artist;
mod resource_artist;
mod vegetation_artist;
mod world_artist;

use crate::world::*;
use commons::edge::*;
use commons::*;
use isometric::Command;
use std::default::Default;

pub use avatar_artist::*;
pub use fast_avatar_artist::*;
pub use world_artist::{Slab, WorldArtist, WorldArtistParameters, WorldColoring};
