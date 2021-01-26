use crate::actors::{
    AvatarArtistActor, AvatarVisibility, AvatarsActor, BasicAvatarControls, BasicRoadBuilder,
    Cheats, Clock, Labels, ObjectBuilder, PathfinderService, PathfindingAvatarControls, PrimeMover,
    RealTime, ResourceTargets, Rotate, SetupNewWorld, SpeedControl, TownBuilderActor,
    TownHouseArtist, TownLabelArtist, VisibilityActor, Voyager, WorldArtistActor,
};
use crate::avatar::AvatarTravelDuration;
use crate::avatars::Avatars;
use crate::game::{Game, GameParams};
use crate::nation::Nation;
use crate::pathfinder::Pathfinder;
use crate::route::Routes;
use crate::settlement::Settlement;
use crate::simulation::Simulation;
use crate::territory::Territory;
use crate::traits::{
    NotMock, PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendAvatars, SendClock,
    SendGame, SendNations, SendParameters, SendPathfinder, SendRotate, SendRoutes, SendSettlements,
    SendSim, SendTerritory, SendTownHouseArtist, SendTownLabelArtist, SendVisibility, SendVoyager,
    SendWorld, SendWorldArtist,
};
use crate::world::World;
use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use commons::V2;
use futures::future::BoxFuture;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Polysender {
    pub game_tx: FnSender<Game>,
    pub avatar_artist_tx: FnSender<AvatarArtistActor<Polysender>>,
    pub avatar_visibility_tx: FnSender<AvatarVisibility<Polysender>>,
    pub avatars_tx: FnSender<AvatarsActor>,
    pub basic_avatar_controls_tx: FnSender<BasicAvatarControls<Polysender>>,
    pub basic_road_builder_tx: FnSender<BasicRoadBuilder<Polysender>>,
    pub cheats_tx: FnSender<Cheats<Polysender>>,
    pub clock_tx: FnSender<Clock<RealTime>>,
    pub labels_tx: FnSender<Labels<Polysender>>,
    pub object_builder_tx: FnSender<ObjectBuilder<Polysender>>,
    pub pathfinding_avatar_controls_tx: FnSender<PathfindingAvatarControls<Polysender>>,
    pub pathfinder_with_planned_roads_tx:
        FnSender<PathfinderService<Polysender, AvatarTravelDuration>>,
    pub pathfinder_without_planned_roads_tx:
        FnSender<PathfinderService<Polysender, AvatarTravelDuration>>,
    pub prime_mover_tx: FnSender<PrimeMover<Polysender>>,
    pub resource_targets_tx: FnSender<ResourceTargets<Polysender>>,
    pub rotate_tx: FnSender<Rotate>,
    pub setup_new_world_tx: FnSender<SetupNewWorld<Polysender>>,
    pub simulation_tx: FnSender<Simulation<Polysender>>,
    pub speed_control_tx: FnSender<SpeedControl<Polysender>>,
    pub town_builder_tx: FnSender<TownBuilderActor<Polysender>>,
    pub town_house_artist_tx: FnSender<TownHouseArtist<Polysender>>,
    pub town_label_artist_tx: FnSender<TownLabelArtist<Polysender>>,
    pub visibility_tx: FnSender<VisibilityActor<Polysender>>,
    pub voyager_tx: FnSender<Voyager<Polysender>>,
    pub world_artist_tx: FnSender<WorldArtistActor<Polysender>>,
}

impl Polysender {
    pub fn clone_with_name(&self, name: &'static str) -> Polysender {
        Polysender {
            game_tx: self.game_tx.clone_with_name(name),
            avatar_artist_tx: self.avatar_artist_tx.clone_with_name(name),
            avatar_visibility_tx: self.avatar_visibility_tx.clone_with_name(name),
            avatars_tx: self.avatars_tx.clone_with_name(name),
            basic_avatar_controls_tx: self.basic_avatar_controls_tx.clone_with_name(name),
            basic_road_builder_tx: self.basic_road_builder_tx.clone_with_name(name),
            cheats_tx: self.cheats_tx.clone_with_name(name),
            clock_tx: self.clock_tx.clone_with_name(name),
            labels_tx: self.labels_tx.clone_with_name(name),
            object_builder_tx: self.object_builder_tx.clone_with_name(name),
            pathfinding_avatar_controls_tx: self
                .pathfinding_avatar_controls_tx
                .clone_with_name(name),
            pathfinder_with_planned_roads_tx: self
                .pathfinder_with_planned_roads_tx
                .clone_with_name(name),
            pathfinder_without_planned_roads_tx: self
                .pathfinder_without_planned_roads_tx
                .clone_with_name(name),
            prime_mover_tx: self.prime_mover_tx.clone_with_name(name),
            resource_targets_tx: self.resource_targets_tx.clone_with_name(name),
            rotate_tx: self.rotate_tx.clone_with_name(name),
            setup_new_world_tx: self.setup_new_world_tx.clone_with_name(name),
            simulation_tx: self.simulation_tx.clone_with_name(name),
            speed_control_tx: self.speed_control_tx.clone_with_name(name),
            town_builder_tx: self.town_builder_tx.clone_with_name(name),
            town_house_artist_tx: self.town_house_artist_tx.clone_with_name(name),
            town_label_artist_tx: self.town_label_artist_tx.clone_with_name(name),
            visibility_tx: self.visibility_tx.clone_with_name(name),
            voyager_tx: self.voyager_tx.clone_with_name(name),
            world_artist_tx: self.world_artist_tx.clone_with_name(name),
        }
    }
}

#[async_trait]
impl SendAvatars for Polysender {
    async fn send_avatars<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Avatars) -> O + Send + 'static,
    {
        self.avatars_tx
            .send(move |avatars| function(&mut avatars.state()))
            .await
    }

    fn send_avatars_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Avatars) -> O + Send + 'static,
    {
        self.avatars_tx
            .send(move |avatars| function(&mut avatars.state()));
    }
}

#[async_trait]
impl SendClock for Polysender {
    type T = RealTime;

    async fn send_clock<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Clock<RealTime>) -> O + Send + 'static,
    {
        self.clock_tx.send(move |clock| function(clock)).await
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
        F: FnOnce(&mut HashMap<String, Nation>) -> O + Send + 'static,
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
        F: FnOnce(&GameParams) -> O + Send + 'static,
    {
        self.game_tx
            .send(move |game| function(&game.game_state().params))
            .await
    }
}

#[async_trait]
impl SendRotate for Polysender {
    fn send_rotate_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Rotate) -> O + Send + 'static,
    {
        self.rotate_tx.send(function);
    }
}

#[async_trait]
impl SendRoutes for Polysender {
    async fn send_routes<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Routes) -> O + Send + 'static,
    {
        self.game_tx
            .send(move |game| function(&mut game.mut_state().routes))
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
        F: FnOnce(&mut Simulation<Self>) -> O + Send + 'static,
    {
        self.simulation_tx.send(function).await
    }

    fn send_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation<Self>) -> O + Send + 'static,
    {
        self.simulation_tx.send(function);
    }
}

#[async_trait]
impl SendTerritory for Polysender {
    async fn send_territory<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Territory) -> O + Send + 'static,
    {
        self.game_tx
            .send(move |game| function(&mut game.mut_state().territory))
            .await
    }
}

#[async_trait]
impl SendTownHouseArtist for Polysender {
    fn send_town_house_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownHouseArtist<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.town_house_artist_tx
            .send_future(move |town_house_artist| function(town_house_artist));
    }
}

#[async_trait]
impl SendTownLabelArtist for Polysender {
    fn send_town_label_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownLabelArtist<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.town_label_artist_tx
            .send_future(move |town_label_artist| function(town_label_artist));
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
    type T = FnSender<PathfinderService<Polysender, AvatarTravelDuration>>;

    fn pathfinder_with_planned_roads(&self) -> &Self::T {
        &self.pathfinder_with_planned_roads_tx
    }
}

impl PathfinderWithoutPlannedRoads for Polysender {
    type T = FnSender<PathfinderService<Polysender, AvatarTravelDuration>>;

    fn pathfinder_without_planned_roads(&self) -> &Self::T {
        &self.pathfinder_without_planned_roads_tx
    }
}

#[async_trait]
impl SendPathfinder for FnSender<PathfinderService<Polysender, AvatarTravelDuration>> {
    type T = AvatarTravelDuration;

    async fn send_pathfinder<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<AvatarTravelDuration>) -> O + Send + 'static,
    {
        self.send(move |service| function(service.pathfinder()))
            .await
    }

    fn send_pathfinder_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<AvatarTravelDuration>) -> O + Send + 'static,
    {
        self.send(move |service| function(service.pathfinder()));
    }
}

impl NotMock for Polysender {}
