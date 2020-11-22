use super::*;

use std::sync::{Arc, RwLock};

use commons::fn_sender::FnSender;

use crate::actors::traits::WithVisibility;
use crate::actors::{VisibilityActor, WorldArtistActor};
use crate::avatar::AvatarTravelDuration;
use crate::game::Game;
use crate::pathfinder::Pathfinder;

#[derive(Clone)]
pub struct Polysender {
    pub game: FnSender<Game>,
    pub visibility: FnSender<VisibilityActor>,
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

#[async_trait]
impl WithVisibility for Polysender {
    async fn with_visibility<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static,
    {
        self.visibility
            .send(move |mut visibility| function(&mut visibility))
            .await
    }

    fn with_visibility_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static,
    {
        self.visibility
            .send(move |mut visibility| function(&mut visibility));
    }
}
