use super::*;

use crate::actors::{VisibilityActor, WorldArtistActor};
use crate::avatar::AvatarTravelDuration;
use crate::game::Game;
use crate::pathfinder::Pathfinder;
use crate::traits::{PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendGame, SendPathfinder, SendVisibility, SendWorld, SendWorldArtist};
use crate::world::World;
use commons::fn_sender::FnSender;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Polysender {
    pub game: FnSender<Game>,
    pub visibility: FnSender<VisibilityActor>,
    pub world_artist: FnSender<WorldArtistActor>,
    pub pathfinder_without_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
    pub pathfinder_with_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
}

impl Polysender {
    pub fn clone_with_name(&self, name: &'static str) -> Polysender {
        Polysender {
            game: self.game.clone_with_name(name),
            visibility: self.visibility.clone_with_name(name),
            world_artist: self.world_artist.clone_with_name(name),
            pathfinder_without_planned_roads: self.pathfinder_without_planned_roads.clone(),
            pathfinder_with_planned_roads: self.pathfinder_with_planned_roads.clone(),
        }
    }
}

#[async_trait]
impl SendGame for Polysender {
    async fn send_game<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Game) -> O + Send + 'static,
    {
        self.game.send(function).await
    }

    fn send_game_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Game) -> O + Send + 'static,
    {
        self.game.send(function);
    }
}

impl SendVisibility for Polysender {
    fn send_visibility_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static,
    {
        self.visibility
            .send(move |mut visibility| function(&mut visibility));
    }
}

#[async_trait]
impl SendWorld for Polysender {
    async fn send_world<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static,
    {
        self.game
            .send(move |game| function(&mut game.mut_state().world))
            .await
    }

    fn send_world_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static,
    {
        self.game
            .send(move |game| function(&mut game.mut_state().world));
    }
}

impl SendWorldArtist for Polysender {
    fn send_world_artist_future_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor) -> commons::future::BoxFuture<O> + Send + 'static,
    {
        self.world_artist
            .send_future(move |world_artist| function(world_artist));
    }
}

impl PathfinderWithPlannedRoads<AvatarTravelDuration, Arc<RwLock<Pathfinder<AvatarTravelDuration>>>>  for Polysender{
    fn pathfinder_with_planned_roads(&mut self) -> Arc<RwLock<Pathfinder<AvatarTravelDuration>>> {
        self.pathfinder_with_planned_roads
    }
}

impl PathfinderWithoutPlannedRoads<AvatarTravelDuration, Arc<RwLock<Pathfinder<AvatarTravelDuration>>>>  for Polysender{
    fn pathfinder_without_planned_roads(&mut self) -> Arc<RwLock<Pathfinder<AvatarTravelDuration>>> {
        self.pathfinder_without_planned_roads
    }
}

#[async_trait]
impl SendPathfinder<AvatarTravelDuration> for Arc<RwLock<Pathfinder<AvatarTravelDuration>>> {
    async fn send_pathfinder<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<AvatarTravelDuration>) -> O + Send + 'static {
        function(&mut self.write().unwrap())
    }

    fn send_pathfinder_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<AvatarTravelDuration>) -> O + Send + 'static {
        function(&mut self.write().unwrap());
    }
}