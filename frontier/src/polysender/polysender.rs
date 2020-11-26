use super::*;

use crate::actors::{VisibilityActor, WorldArtistActor};
use crate::avatar::AvatarTravelDuration;
use crate::game::Game;
use crate::pathfinder::Pathfinder;
use crate::traits::{
    PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendGame, SendPathfinder,
    SendVisibility, SendWorld, SendWorldArtist,
};
use crate::world::World;
use commons::fn_sender::{fn_channel, FnReceiver, FnSender};
use std::sync::{Arc, RwLock};

struct Channel<T>
where
    T: Send,
{
    rx: FnReceiver<T>,
    tx: FnSender<T>,
}

impl<T> Channel<T>
where
    T: Send,
{
    fn new() -> Channel<T> {
        let (tx, rx) = fn_channel();
        Channel { tx, rx }
    }

    fn clone_with_name(&self, name: &'static str) -> Channel<T> {
        Channel {
            tx: self.tx.clone_with_name(name),
            rx: self.rx.clone(),
        }
    }
}

impl<T> Clone for Channel<T>
where
    T: Send,
{
    fn clone(&self) -> Self {
        Channel {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Polysender {
    game_tx: FnSender<Game>,
    visibility: Channel<VisibilityActor>,
    world_artist: Channel<WorldArtistActor>,
    pathfinder_with_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
    pathfinder_without_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
}

impl Polysender {
    pub fn new(
        game_tx: FnSender<Game>,
        pathfinder_with_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
        pathfinder_without_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
    ) -> Polysender {
        Polysender {
            game_tx,
            visibility: Channel::new(),
            world_artist: Channel::new(),
            pathfinder_with_planned_roads,
            pathfinder_without_planned_roads,
        }
    }

    pub fn clone_with_name(&self, name: &'static str) -> Polysender {
        Polysender {
            game_tx: self.game_tx.clone_with_name(name),
            visibility: self.visibility.clone_with_name(name),
            world_artist: self.world_artist.clone_with_name(name),
            pathfinder_with_planned_roads: self.pathfinder_with_planned_roads.clone(),
            pathfinder_without_planned_roads: self.pathfinder_without_planned_roads.clone(),
        }
    }

    pub fn visibility_rx(&self) -> FnReceiver<VisibilityActor> {
        self.visibility.rx.clone()
    }

    pub fn world_artist_rx(&self) -> FnReceiver<WorldArtistActor> {
        self.world_artist.rx.clone()
    }
}

#[async_trait]
impl SendGame for Polysender {
    async fn send_game<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Game) -> O + Send + 'static,
    {
        self.game_tx.send(function).await
    }

    fn send_game_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Game) -> O + Send + 'static,
    {
        self.game_tx.send(function);
    }
}

impl SendVisibility for Polysender {
    fn send_visibility_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static,
    {
        self.visibility
            .tx
            .send(move |mut visibility| function(&mut visibility));
    }
}

#[async_trait]
impl SendWorld for Polysender {
    async fn send_world<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static,
    {
        self.game_tx
            .send(move |game| function(&mut game.mut_state().world))
            .await
    }

    fn send_world_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static,
    {
        self.game_tx
            .send(move |game| function(&mut game.mut_state().world));
    }
}

impl SendWorldArtist for Polysender {
    fn send_world_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor) -> commons::future::BoxFuture<O> + Send + 'static,
    {
        self.world_artist
            .tx
            .send_future(move |world_artist| function(world_artist));
    }
}

impl PathfinderWithPlannedRoads for Polysender {
    type T = Arc<RwLock<Pathfinder<AvatarTravelDuration>>>;

    fn pathfinder_with_planned_roads(&self) -> &Self::T {
        &self.pathfinder_with_planned_roads
    }
}

impl PathfinderWithoutPlannedRoads for Polysender {
    type T = Arc<RwLock<Pathfinder<AvatarTravelDuration>>>;

    fn pathfinder_without_planned_roads(&self) -> &Self::T {
        &self.pathfinder_without_planned_roads
    }
}

#[async_trait]
impl SendPathfinder for Arc<RwLock<Pathfinder<AvatarTravelDuration>>> {
    type T = AvatarTravelDuration;

    async fn send_pathfinder<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<AvatarTravelDuration>) -> O + Send + 'static,
    {
        function(&mut self.write().unwrap())
    }

    fn send_pathfinder_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<AvatarTravelDuration>) -> O + Send + 'static,
    {
        function(&mut self.write().unwrap());
    }
}
