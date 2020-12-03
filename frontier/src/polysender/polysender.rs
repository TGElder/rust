use super::*;

use crate::actors::{TownHouses, VisibilityActor, Voyager, WorldArtistActor};
use crate::avatar::AvatarTravelDuration;
use crate::game::Game;
use crate::pathfinder::Pathfinder;
use crate::settlement::Settlement;
use crate::simulation::Simulation;
use crate::traits::{
    PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendGame, SendNations,
    SendParameters, SendPathfinder, SendSettlements, SendSim, SendTownHouses, SendVisibility,
    SendVoyager, SendWorld, SendWorldArtist,
};
use crate::world::World;
use commons::fn_sender::FnSender;
use commons::futures::future::BoxFuture;
use commons::V2;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Polysender {
    game_tx: FnSender<Game>,
    simulation_tx: FnSender<Simulation>,
    town_houses_tx: FnSender<TownHouses<Polysender>>,
    visibility_tx: FnSender<VisibilityActor<Polysender>>,
    voyager_tx: FnSender<Voyager<Polysender>>,
    world_artist_tx: FnSender<WorldArtistActor<Polysender>>,
    pathfinder_with_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
    pathfinder_without_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
}

impl Polysender {
    pub fn new(
        game_tx: FnSender<Game>,
        simulation_tx: FnSender<Simulation>,
        town_houses_tx: FnSender<TownHouses<Polysender>>,
        visibility_tx: FnSender<VisibilityActor<Polysender>>,
        voyager_tx: FnSender<Voyager<Polysender>>,
        world_artist_tx: FnSender<WorldArtistActor<Polysender>>,
        pathfinder_with_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
        pathfinder_without_planned_roads: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
    ) -> Polysender {
        Polysender {
            game_tx,
            simulation_tx,
            town_houses_tx,
            visibility_tx,
            voyager_tx,
            world_artist_tx,
            pathfinder_with_planned_roads,
            pathfinder_without_planned_roads,
        }
    }

    pub fn clone_with_name(&self, name: &'static str) -> Polysender {
        Polysender {
            game_tx: self.game_tx.clone_with_name(name),
            simulation_tx: self.simulation_tx.clone_with_name(name),
            town_houses_tx: self.town_houses_tx.clone_with_name(name),
            visibility_tx: self.visibility_tx.clone_with_name(name),
            voyager_tx: self.voyager_tx.clone_with_name(name),
            world_artist_tx: self.world_artist_tx.clone_with_name(name),
            pathfinder_with_planned_roads: self.pathfinder_with_planned_roads.clone(),
            pathfinder_without_planned_roads: self.pathfinder_without_planned_roads.clone(),
        }
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

#[async_trait]
impl SendNations for Polysender {
    async fn send_nations<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut HashMap<String, crate::nation::Nation>) -> O + Send + 'static,
    {
        self.game_tx
            .send(move |game| function(&mut game.mut_state().nations))
            .await
    }
}

#[async_trait]
impl SendParameters for Polysender {
    async fn send_parameters<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&crate::game::GameParams) -> O + Send + 'static,
    {
        self.game_tx
            .send(move |game| function(&game.game_state().params))
            .await
    }
}

#[async_trait]
impl SendSettlements for Polysender {
    async fn send_settlements<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut HashMap<V2<usize>, Settlement>) -> O + Send + 'static,
    {
        self.game_tx
            .send(move |game| function(&mut game.mut_state().settlements))
            .await
    }
}

#[async_trait]
impl SendSim for Polysender {
    async fn send_sim<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation) -> O + Send + 'static,
    {
        self.simulation_tx.send(function).await
    }

    fn send_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation) -> O + Send + 'static,
    {
        self.simulation_tx.send(function);
    }
}

#[async_trait]
impl SendTownHouses for Polysender {
    fn send_town_houses_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownHouses<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.town_houses_tx
            .send_future(move |town_houses| function(town_houses));
    }
}

impl SendVoyager for Polysender {
    fn send_voyager_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Voyager<Polysender>) -> BoxFuture<O> + Send + 'static,
    {
        self.voyager_tx
            .send_future(move |voyager| function(voyager));
    }
}

impl SendVisibility for Polysender {
    fn send_visibility_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Polysender>) -> O + Send + 'static,
    {
        self.visibility_tx
            .send(move |mut visibility| function(&mut visibility));
    }

    fn send_visibility_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.visibility_tx
            .send_future(move |visibility| function(visibility));
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
        F: FnOnce(&mut WorldArtistActor<Polysender>) -> BoxFuture<O> + Send + 'static,
    {
        self.world_artist_tx
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
