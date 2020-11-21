use std::sync::{Arc, RwLock};

use commons::fn_sender::FnSender;

use crate::actors::{Visibility, WorldArtistActor};
use crate::avatar::AvatarTravelDuration;
use crate::game::Game;
use crate::pathfinder::Pathfinder;

#[derive(Clone)]
pub struct Polysender {
    pub game: FnSender<Game>,
    pub visibility: FnSender<Visibility>,
    pub world_artist: FnSender<WorldArtistActor>,
    pub pathfinders: Vec<Arc<RwLock<Pathfinder<AvatarTravelDuration>>>>,
}

impl Polysender {
    pub fn clone_with_name(&self, name: &'static str) -> Polysender {
        Polysender {
            game: self.game.clone_with_name(name),
            visibility: self.visibility.clone_with_name(name),
            world_artist: self.world_artist.clone_with_name(name),
            pathfinders: self.pathfinders.clone(),
        }
    }
}
