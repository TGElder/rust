use crate::actors::{
    AvatarVisibility, BasicAvatarControls, BasicRoadBuilder, BridgeBuilderActor, BuilderActor,
    Cheats, ControllersActor, FollowAvatar, Labels, ObjectBuilderActor, PathfindingAvatarControls,
    PrimeMover, ResourceGenActor, ResourceTargets, RiverExplorer, Rotate, SetupNewWorld,
    SetupPathfinders, SetupVisibility, SpeedControl, TownBuilderActor, TownHouseArtist,
    TownLabelArtist, Voyager, WorldArtistActor, WorldGen,
};
use crate::avatar::AvatarTravelDuration;
use crate::avatars::Avatars;
use crate::bridge::Bridges;
use crate::build::BuildQueue;
use crate::nation::Nation;
use crate::parameters::Parameters;
use crate::pathfinder::Pathfinder;
use crate::resource::Resources;
use crate::road_builder::RoadBuildTravelDuration;
use crate::route::{RouteKey, Routes};
use crate::services::clock::{Clock, RealTime};
use crate::services::{BackgroundService, VisibilityService};
use crate::settlement::Settlement;
use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::simulation::settlement::SettlementSimulation;
use crate::system::System;
use crate::territory::{Controllers, Territory};
use crate::traffic::{EdgeTraffic, Traffic};
use crate::traits::has::{HasFollowAvatar, HasParameters};
use crate::traits::{
    NotMock, PathfinderForPlayer, PathfinderForRoutes, RunInBackground, SendEdgeBuildSim,
    SendEngineCommands, SendPositionBuildSim, SendResourceTargets, SendRotate, SendSystem,
    SendTownHouseArtist, SendTownLabelArtist, SendVoyager, SendWorldArtist, WithAvatars,
    WithBridges, WithBuildQueue, WithClock, WithControllers, WithEdgeTraffic, WithNations,
    WithPathfinder, WithResources, WithRouteToPorts, WithRoutes, WithSettlements, WithSimQueue,
    WithTerritory, WithTraffic, WithVisibility, WithVisited, WithWorld,
};
use crate::visited::Visited;
use crate::world::World;
use commons::async_channel::Sender;
use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use commons::V2;
use futures::executor::ThreadPool;
use futures::future::BoxFuture;
use futures::Future;
use isometric::Command;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Context {
    pub avatar_visibility_tx: FnSender<AvatarVisibility<Context>>,
    pub avatars: Arc<RwLock<Avatars>>,
    pub background_service: Arc<BackgroundService>,
    pub basic_avatar_controls_tx: FnSender<BasicAvatarControls<Context>>,
    pub basic_road_builder_tx: FnSender<BasicRoadBuilder<Context>>,
    pub bridge_builder_tx: FnSender<BridgeBuilderActor<Context>>,
    pub bridges: Arc<RwLock<Bridges>>,
    pub builder_tx: FnSender<BuilderActor<Context>>,
    pub build_queue: Arc<RwLock<BuildQueue>>,
    pub cheats_tx: FnSender<Cheats<Context>>,
    pub controllers: Arc<RwLock<Controllers>>,
    pub controllers_tx: FnSender<ControllersActor<Context>>,
    pub clock: Arc<RwLock<Clock<RealTime>>>,
    pub edge_sim_tx: FnSender<EdgeBuildSimulation<Context, RoadBuildTravelDuration>>,
    pub edge_traffic: Arc<RwLock<EdgeTraffic>>,
    pub engine_tx: Sender<Vec<Command>>,
    pub follow_avatar: Arc<RwLock<bool>>,
    pub follow_avatar_tx: FnSender<FollowAvatar<Context>>,
    pub labels_tx: FnSender<Labels<Context>>,
    pub nations: Arc<RwLock<HashMap<String, Nation>>>,
    pub object_builder_tx: FnSender<ObjectBuilderActor<Context>>,
    pub parameters: Arc<Parameters>,
    pub player_pathfinder: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
    pub pathfinding_avatar_controls_tx: FnSender<PathfindingAvatarControls<Context>>,
    pub pool: ThreadPool,
    pub position_sim_tx: FnSender<PositionBuildSimulation<Context>>,
    pub prime_mover_tx: FnSender<PrimeMover<Context>>,
    pub resource_gen_tx: FnSender<ResourceGenActor<Context>>,
    pub resource_targets_tx: FnSender<ResourceTargets<Context>>,
    pub resources: Arc<RwLock<Resources>>,
    pub river_explorer_tx: FnSender<RiverExplorer<Context>>,
    pub rotate_tx: FnSender<Rotate<Context>>,
    pub route_to_ports: Arc<RwLock<HashMap<RouteKey, HashSet<V2<usize>>>>>,
    pub routes: Arc<RwLock<Routes>>,
    pub routes_pathfinder: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
    pub settlement_sim_txs: Vec<FnSender<SettlementSimulation<Context, AvatarTravelDuration>>>,
    pub settlements: Arc<RwLock<HashMap<V2<usize>, Settlement>>>,
    pub setup_new_world_tx: FnSender<SetupNewWorld<Context>>,
    pub setup_pathfinders_tx: FnSender<SetupPathfinders<Context>>,
    pub setup_visibility_tx: FnSender<SetupVisibility<Context>>,
    pub sim_queue: Arc<RwLock<Vec<V2<usize>>>>,
    pub speed_control_tx: FnSender<SpeedControl<Context>>,
    pub system_tx: FnSender<System>,
    pub territory: Arc<RwLock<Territory>>,
    pub town_builder_tx: FnSender<TownBuilderActor<Context>>,
    pub town_house_artist_tx: FnSender<TownHouseArtist<Context>>,
    pub town_label_artist_tx: FnSender<TownLabelArtist<Context>>,
    pub traffic: Arc<RwLock<Traffic>>,
    pub visibility: Arc<RwLock<VisibilityService>>,
    pub visited: Arc<RwLock<Visited>>,
    pub voyager_tx: FnSender<Voyager<Context>>,
    pub world: Arc<RwLock<World>>,
    pub world_artist_tx: FnSender<WorldArtistActor<Context>>,
    pub world_gen_tx: FnSender<WorldGen<Context>>,
}

impl Context {
    pub fn clone_with_name(&self, name: &'static str) -> Context {
        Context {
            avatar_visibility_tx: self.avatar_visibility_tx.clone_with_name(name),
            avatars: self.avatars.clone(),
            background_service: self.background_service.clone(),
            basic_avatar_controls_tx: self.basic_avatar_controls_tx.clone_with_name(name),
            basic_road_builder_tx: self.basic_road_builder_tx.clone_with_name(name),
            bridge_builder_tx: self.bridge_builder_tx.clone_with_name(name),
            bridges: self.bridges.clone(),
            builder_tx: self.builder_tx.clone_with_name(name),
            build_queue: self.build_queue.clone(),
            cheats_tx: self.cheats_tx.clone_with_name(name),
            clock: self.clock.clone(),
            edge_sim_tx: self.edge_sim_tx.clone(),
            edge_traffic: self.edge_traffic.clone(),
            engine_tx: self.engine_tx.clone(),
            follow_avatar: self.follow_avatar.clone(),
            follow_avatar_tx: self.follow_avatar_tx.clone(),
            controllers: self.controllers.clone(),
            controllers_tx: self.controllers_tx.clone(),
            labels_tx: self.labels_tx.clone_with_name(name),
            nations: self.nations.clone(),
            object_builder_tx: self.object_builder_tx.clone_with_name(name),
            parameters: self.parameters.clone(),
            player_pathfinder: self.player_pathfinder.clone(),
            pathfinding_avatar_controls_tx: self
                .pathfinding_avatar_controls_tx
                .clone_with_name(name),
            pool: self.pool.clone(),
            position_sim_tx: self.position_sim_tx.clone_with_name(name),
            prime_mover_tx: self.prime_mover_tx.clone_with_name(name),
            resource_gen_tx: self.resource_gen_tx.clone_with_name(name),
            resource_targets_tx: self.resource_targets_tx.clone_with_name(name),
            resources: self.resources.clone(),
            river_explorer_tx: self.river_explorer_tx.clone_with_name(name),
            rotate_tx: self.rotate_tx.clone_with_name(name),
            route_to_ports: self.route_to_ports.clone(),
            routes: self.routes.clone(),
            routes_pathfinder: self.routes_pathfinder.clone(),
            settlement_sim_txs: self
                .settlement_sim_txs
                .iter()
                .map(|cx| cx.clone_with_name(name))
                .collect(),
            settlements: self.settlements.clone(),
            setup_new_world_tx: self.setup_new_world_tx.clone_with_name(name),
            setup_pathfinders_tx: self.setup_pathfinders_tx.clone_with_name(name),
            setup_visibility_tx: self.setup_visibility_tx.clone_with_name(name),
            sim_queue: self.sim_queue.clone(),
            speed_control_tx: self.speed_control_tx.clone_with_name(name),
            system_tx: self.system_tx.clone_with_name(name),
            territory: self.territory.clone(),
            traffic: self.traffic.clone(),
            town_builder_tx: self.town_builder_tx.clone_with_name(name),
            town_house_artist_tx: self.town_house_artist_tx.clone_with_name(name),
            town_label_artist_tx: self.town_label_artist_tx.clone_with_name(name),
            visibility: self.visibility.clone(),
            visited: self.visited.clone(),
            voyager_tx: self.voyager_tx.clone_with_name(name),
            world: self.world.clone(),
            world_artist_tx: self.world_artist_tx.clone_with_name(name),
            world_gen_tx: self.world_gen_tx.clone_with_name(name),
        }
    }
}

#[async_trait]
impl HasFollowAvatar for Context {
    async fn follow_avatar(&self) -> bool {
        *self.follow_avatar.read().await
    }

    async fn set_follow_avatar(&self, value: bool) {
        *self.follow_avatar.write().await = value;
    }
}

impl HasParameters for Context {
    fn parameters(&self) -> &Parameters {
        self.parameters.as_ref()
    }
}

impl RunInBackground for Context {
    fn run_in_background<Fut>(&self, future: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.background_service.run_in_background(future);
    }
}

#[async_trait]
impl SendEdgeBuildSim for Context {
    type D = RoadBuildTravelDuration;

    async fn send_edge_build_sim_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut EdgeBuildSimulation<Self, Self::D>) -> BoxFuture<O> + Send + 'static,
    {
        self.edge_sim_tx.send_future(function).await
    }
}

#[async_trait]
impl SendEngineCommands for Context {
    async fn send_engine_commands(&self, commands: Vec<Command>) {
        self.engine_tx.send(commands).await.unwrap()
    }
}

#[async_trait]
impl SendPositionBuildSim for Context {
    async fn send_position_build_sim_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut PositionBuildSimulation<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.position_sim_tx.send_future(function).await
    }

    fn send_position_build_sim_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut PositionBuildSimulation<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.position_sim_tx.send_future(function);
    }
}

#[async_trait]
impl SendResourceTargets for Context {
    async fn send_resource_targets_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut ResourceTargets<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.resource_targets_tx.send_future(function).await
    }
}

#[async_trait]
impl SendRotate for Context {
    fn send_rotate_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Rotate<Self>) -> O + Send + 'static,
    {
        self.rotate_tx.send(function);
    }
}

#[async_trait]
impl SendSystem for Context {
    async fn send_system<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut System) -> O + Send + 'static,
    {
        self.system_tx.send(move |system| function(system)).await
    }

    async fn send_system_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut super::System) -> BoxFuture<O> + Send + 'static,
    {
        self.system_tx.send_future(function).await
    }
}

#[async_trait]
impl SendTownHouseArtist for Context {
    fn send_town_house_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownHouseArtist<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.town_house_artist_tx.send_future(function);
    }
}

#[async_trait]
impl SendTownLabelArtist for Context {
    fn send_town_label_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownLabelArtist<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.town_label_artist_tx.send_future(function);
    }
}

impl SendVoyager for Context {
    fn send_voyager_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Voyager<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.voyager_tx.send_future(function);
    }
}

impl SendWorldArtist for Context {
    fn send_world_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor<Self>) -> BoxFuture<O> + Send + 'static,
    {
        self.world_artist_tx.send_future(function);
    }
}

#[async_trait]
impl WithAvatars for Context {
    async fn with_avatars<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Avatars) -> O + Send,
    {
        let avatars = self.avatars.read().await;
        function(&avatars)
    }

    async fn mut_avatars<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Avatars) -> O + Send,
    {
        let mut avatars = self.avatars.write().await;
        function(&mut avatars)
    }
}

#[async_trait]
impl WithBridges for Context {
    async fn with_bridges<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Bridges) -> O + Send,
    {
        let bridges = self.bridges.read().await;
        function(&bridges)
    }

    async fn mut_bridges<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Bridges) -> O + Send,
    {
        let mut bridges = self.bridges.write().await;
        function(&mut bridges)
    }
}

#[async_trait]
impl WithBuildQueue for Context {
    async fn with_build_queue<F, O>(&self, function: F) -> O
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
impl WithClock for Context {
    type T = RealTime;

    async fn with_clock<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Clock<RealTime>) -> O + Send,
    {
        let clock = self.clock.read().await;
        function(&clock)
    }

    async fn mut_clock<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Clock<RealTime>) -> O + Send,
    {
        let mut clock = self.clock.write().await;
        function(&mut clock)
    }
}

#[async_trait]
impl WithControllers for Context {
    async fn with_controllers<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Controllers) -> O + Send,
    {
        let controllers = self.controllers.read().await;
        function(&controllers)
    }

    async fn mut_controllers<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Controllers) -> O + Send,
    {
        let mut controllers = self.controllers.write().await;
        function(&mut controllers)
    }
}
#[async_trait]
impl WithEdgeTraffic for Context {
    async fn with_edge_traffic<F, O>(&self, function: F) -> O
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
impl WithNations for Context {
    async fn with_nations<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&HashMap<String, Nation>) -> O + Send,
    {
        let nations = self.nations.read().await;
        function(&nations)
    }

    async fn mut_nations<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut HashMap<String, Nation>) -> O + Send,
    {
        let mut nations = self.nations.write().await;
        function(&mut nations)
    }
}

#[async_trait]
impl WithResources for Context {
    async fn with_resources<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Resources) -> O + Send,
    {
        let resources = self.resources.read().await;
        function(&resources)
    }

    async fn mut_resources<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Resources) -> O + Send,
    {
        let mut resources = self.resources.write().await;
        function(&mut resources)
    }
}

#[async_trait]
impl WithRoutes for Context {
    async fn with_routes<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Routes) -> O + Send,
    {
        let routes = self.routes.read().await;
        function(&routes)
    }

    async fn mut_routes<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Routes) -> O + Send,
    {
        let mut routes = self.routes.write().await;
        function(&mut routes)
    }
}

#[async_trait]
impl WithRouteToPorts for Context {
    async fn with_route_to_ports<F, O>(&self, function: F) -> O
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
impl WithSettlements for Context {
    async fn with_settlements<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&HashMap<V2<usize>, Settlement>) -> O + Send,
    {
        let settlements = self.settlements.read().await;
        function(&settlements)
    }

    async fn mut_settlements<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut HashMap<V2<usize>, Settlement>) -> O + Send,
    {
        let mut settlements = self.settlements.write().await;
        function(&mut settlements)
    }
}

#[async_trait]
impl WithSimQueue for Context {
    async fn with_sim_queue<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Vec<V2<usize>>) -> O + Send,
    {
        let sim_queue = self.sim_queue.read().await;
        function(&sim_queue)
    }

    async fn mut_sim_queue<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Vec<V2<usize>>) -> O + Send,
    {
        let mut sim_queue = self.sim_queue.write().await;
        function(&mut sim_queue)
    }
}

#[async_trait]
impl WithTerritory for Context {
    async fn with_territory<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Territory) -> O + Send,
    {
        let territory = self.territory.read().await;
        function(&territory)
    }

    async fn mut_territory<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Territory) -> O + Send,
    {
        let mut territory = self.territory.write().await;
        function(&mut territory)
    }
}

#[async_trait]
impl WithTraffic for Context {
    async fn with_traffic<F, O>(&self, function: F) -> O
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

#[async_trait]
impl WithVisited for Context {
    async fn with_visited<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Visited) -> O + Send,
    {
        let visited = self.visited.read().await;
        function(&visited)
    }

    async fn mut_visited<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Visited) -> O + Send,
    {
        let mut visited = self.visited.write().await;
        function(&mut visited)
    }
}

#[async_trait]
impl WithVisibility for Context {
    async fn with_visibility<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&VisibilityService) -> O + Send,
    {
        let visibility = self.visibility.read().await;
        function(&visibility)
    }

    async fn mut_visibility<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut VisibilityService) -> O + Send,
    {
        let mut visibility = self.visibility.write().await;
        function(&mut visibility)
    }
}

#[async_trait]
impl WithWorld for Context {
    async fn with_world<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&World) -> O + Send,
    {
        let world = self.world.read().await;
        function(&world)
    }

    async fn mut_world<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut World) -> O + Send,
    {
        let mut world = self.world.write().await;
        function(&mut world)
    }
}

#[async_trait]
impl WithPathfinder for Arc<RwLock<Pathfinder<AvatarTravelDuration>>> {
    type T = AvatarTravelDuration;

    async fn with_pathfinder<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Pathfinder<Self::T>) -> O + Send,
    {
        let pathfinder = self.read().await;
        function(&pathfinder)
    }

    async fn mut_pathfinder<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Pathfinder<Self::T>) -> O + Send,
    {
        let mut pathfinder = self.write().await;
        function(&mut pathfinder)
    }
}

impl PathfinderForRoutes for Context {
    type T = Arc<RwLock<Pathfinder<AvatarTravelDuration>>>;

    fn routes_pathfinder(&self) -> &Self::T {
        &self.routes_pathfinder
    }
}

impl PathfinderForPlayer for Context {
    type T = Arc<RwLock<Pathfinder<AvatarTravelDuration>>>;

    fn player_pathfinder(&self) -> &Self::T {
        &self.player_pathfinder
    }
}

impl NotMock for Context {}
