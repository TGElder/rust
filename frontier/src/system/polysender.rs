use crate::actors::{
    AvatarArtistActor, AvatarVisibility, AvatarsActor, BasicAvatarControls, BasicRoadBuilder,
    BuilderActor, Cheats, Clock, Labels, Nations, ObjectBuilder, PathfinderService,
    PathfindingAvatarControls, PrimeMover, RealTime, ResourceTargets, Rotate, RoutesActor,
    Settlements, SetupNewWorld, SpeedControl, TerritoryActor, TownBuilderActor, TownHouseArtist,
    TownLabelArtist, VisibilityActor, Voyager, WorldActor, WorldArtistActor,
};
use crate::avatar::AvatarTravelDuration;
use crate::avatars::Avatars;
use crate::build::BuildQueue;
use crate::nation::Nation;
use crate::parameters::Parameters;
use crate::pathfinder::Pathfinder;
use crate::route::{RouteKey, Routes};
use crate::settlement::Settlement;
use crate::simulation::{EdgeTraffic, Simulation, Traffic};
use crate::territory::Territory;
use crate::traits::{
    NotMock, PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendAvatars, SendClock,
    SendNations, SendParameters, SendPathfinder, SendRotate, SendRoutes, SendSettlements, SendSim,
    SendTerritory, SendTownHouseArtist, SendTownLabelArtist, SendVisibility, SendVoyager,
    SendWorld, SendWorldArtist,
};
use crate::traits::{WithBuildQueue, WithEdgeTraffic, WithRouteToPorts, WithTraffic};
use crate::world::World;
use commons::async_std::sync::RwLock;
use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use commons::V2;
use futures::future::BoxFuture;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct Polysender {
    pub avatar_artist_tx: FnSender<AvatarArtistActor<Polysender>>,
    pub avatar_visibility_tx: FnSender<AvatarVisibility<Polysender>>,
    pub avatars_tx: FnSender<AvatarsActor>,
    pub basic_avatar_controls_tx: FnSender<BasicAvatarControls<Polysender>>,
    pub basic_road_builder_tx: FnSender<BasicRoadBuilder<Polysender>>,
    pub builder_tx: FnSender<BuilderActor<Polysender>>,
    pub build_queue: Arc<RwLock<BuildQueue>>,
    pub cheats_tx: FnSender<Cheats<Polysender>>,
    pub clock_tx: FnSender<Clock<RealTime>>,
    pub edge_traffic: Arc<RwLock<EdgeTraffic>>,
    pub labels_tx: FnSender<Labels<Polysender>>,
    pub nations_tx: FnSender<Nations>,
    pub object_builder_tx: FnSender<ObjectBuilder<Polysender>>,
    pub parameters_tx: FnSender<Parameters>,
    pub pathfinding_avatar_controls_tx: FnSender<PathfindingAvatarControls<Polysender>>,
    pub pathfinder_with_planned_roads_tx:
        FnSender<PathfinderService<Polysender, AvatarTravelDuration>>,
    pub pathfinder_without_planned_roads_tx:
        FnSender<PathfinderService<Polysender, AvatarTravelDuration>>,
    pub prime_mover_tx: FnSender<PrimeMover<Polysender>>,
    pub resource_targets_tx: FnSender<ResourceTargets<Polysender>>,
    pub rotate_tx: FnSender<Rotate>,
    pub routes_tx: FnSender<RoutesActor>,
    pub route_to_ports: Arc<RwLock<HashMap<RouteKey, HashSet<V2<usize>>>>>,
    pub settlements_tx: FnSender<Settlements>,
    pub setup_new_world_tx: FnSender<SetupNewWorld<Polysender>>,
    pub simulation_tx: FnSender<Simulation<Polysender>>,
    pub speed_control_tx: FnSender<SpeedControl<Polysender>>,
    pub territory_tx: FnSender<TerritoryActor>,
    pub town_builder_tx: FnSender<TownBuilderActor<Polysender>>,
    pub town_house_artist_tx: FnSender<TownHouseArtist<Polysender>>,
    pub town_label_artist_tx: FnSender<TownLabelArtist<Polysender>>,
    pub traffic: Arc<RwLock<Traffic>>,
    pub visibility_tx: FnSender<VisibilityActor<Polysender>>,
    pub voyager_tx: FnSender<Voyager<Polysender>>,
    pub world_tx: FnSender<WorldActor<Polysender>>,
    pub world_artist_tx: FnSender<WorldArtistActor<Polysender>>,
}

impl Polysender {
    pub fn clone_with_name(&self, name: &'static str) -> Polysender {
        Polysender {
            avatar_artist_tx: self.avatar_artist_tx.clone_with_name(name),
            avatar_visibility_tx: self.avatar_visibility_tx.clone_with_name(name),
            avatars_tx: self.avatars_tx.clone_with_name(name),
            basic_avatar_controls_tx: self.basic_avatar_controls_tx.clone_with_name(name),
            basic_road_builder_tx: self.basic_road_builder_tx.clone_with_name(name),
            builder_tx: self.builder_tx.clone_with_name(name),
            build_queue: self.build_queue.clone(),
            cheats_tx: self.cheats_tx.clone_with_name(name),
            clock_tx: self.clock_tx.clone_with_name(name),
            edge_traffic: self.edge_traffic.clone(),
            labels_tx: self.labels_tx.clone_with_name(name),
            nations_tx: self.nations_tx.clone_with_name(name),
            object_builder_tx: self.object_builder_tx.clone_with_name(name),
            parameters_tx: self.parameters_tx.clone_with_name(name),
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
            routes_tx: self.routes_tx.clone_with_name(name),
            route_to_ports: self.route_to_ports.clone(),
            settlements_tx: self.settlements_tx.clone_with_name(name),
            setup_new_world_tx: self.setup_new_world_tx.clone_with_name(name),
            simulation_tx: self.simulation_tx.clone_with_name(name),
            speed_control_tx: self.speed_control_tx.clone_with_name(name),
            territory_tx: self.territory_tx.clone_with_name(name),
            traffic: self.traffic.clone(),
            town_builder_tx: self.town_builder_tx.clone_with_name(name),
            town_house_artist_tx: self.town_house_artist_tx.clone_with_name(name),
            town_label_artist_tx: self.town_label_artist_tx.clone_with_name(name),
            visibility_tx: self.visibility_tx.clone_with_name(name),
            voyager_tx: self.voyager_tx.clone_with_name(name),
            world_tx: self.world_tx.clone_with_name(name),
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
impl SendNations for Polysender {
    async fn send_nations<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut HashMap<String, Nation>) -> O + Send + 'static,
    {
        self.nations_tx
            .send(move |nations| function(&mut nations.state()))
            .await
    }
}

#[async_trait]
impl SendParameters for Polysender {
    async fn send_parameters<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&Parameters) -> O + Send + 'static,
    {
        self.parameters_tx
            .send(move |params| function(params))
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
        self.routes_tx
            .send(move |routes| function(&mut routes.state()))
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
        self.settlements_tx
            .send(move |settlements| function(&mut settlements.state()))
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
        self.territory_tx
            .send(move |territory| function(&mut territory.state()))
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
        self.world_tx
            .send(move |world| function(&mut world.state()))
            .await
    }

    fn send_world_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static,
    {
        self.world_tx
            .send(move |world| function(&mut world.state()));
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

#[async_trait]
impl WithBuildQueue for Polysender {
    async fn get_build_queue<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&BuildQueue) -> O + Send,
    {
        let build_queue = self.build_queue.read().await;
        function(&build_queue)
    }

    async fn mut_build_queue<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut BuildQueue) -> O + Send,
    {
        let mut build_queue = self.build_queue.write().await;
        function(&mut build_queue)
    }
}

#[async_trait]
impl WithEdgeTraffic for Polysender {
    async fn get_edge_traffic<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&EdgeTraffic) -> O + Send,
    {
        let edge_traffic = self.edge_traffic.read().await;
        function(&edge_traffic)
    }

    async fn mut_edge_traffic<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut EdgeTraffic) -> O + Send,
    {
        let mut edge_traffic = self.edge_traffic.write().await;
        function(&mut edge_traffic)
    }
}

#[async_trait]
impl WithRouteToPorts for Polysender {
    async fn get_route_to_ports<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send,
    {
        let route_to_ports = self.route_to_ports.read().await;
        function(&route_to_ports)
    }

    async fn mut_route_to_ports<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send,
    {
        let mut route_to_ports = self.route_to_ports.write().await;
        function(&mut route_to_ports)
    }
}

#[async_trait]
impl WithTraffic for Polysender {
    async fn get_traffic<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Traffic) -> O + Send,
    {
        let traffic = self.traffic.read().await;
        function(&traffic)
    }

    async fn mut_traffic<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Traffic) -> O + Send,
    {
        let mut traffic = self.traffic.write().await;
        function(&mut traffic)
    }
}
