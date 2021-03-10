use std::collections::HashSet;
use std::sync::Arc;

use commons::async_std::sync::RwLock;
use commons::fn_sender::{fn_channel, FnMessageExt, FnReceiver};
use commons::persistence::{Load, Save};
use commons::M;
use futures::executor::ThreadPool;
use futures::future::{join_all, FutureExt, RemoteHandle};
use isometric::event_handlers::ZoomHandler;
use isometric::IsometricEngine;

use crate::actors::{
    AvatarArtistActor, AvatarVisibility, BasicAvatarControls, BasicRoadBuilder, BuilderActor,
    Cheats, Labels, ObjectBuilder, PathfindingAvatarControls, PrimeMover, ResourceTargets, Rotate,
    SetupNewWorld, SetupPathfinders, SetupVisibility, SpeedControl, TownBuilderActor,
    TownHouseArtist, TownLabelArtist, Voyager, WorldArtistActor, WorldColoringParameters, WorldGen,
};
use crate::artists::{AvatarArtist, AvatarArtistParams, WorldArtist, WorldArtistParameters};
use crate::avatar::AvatarTravelDuration;
use crate::build::builders::{CropsBuilder, RoadBuilder, TownBuilder};
use crate::parameters::Parameters;
use crate::pathfinder::Pathfinder;
use crate::road_builder::AutoRoadTravelDuration;
use crate::services::clock::{Clock, RealTime};
use crate::services::{BackgroundService, VisibilityService};
use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::simulation::settlement::SettlementSimulation;
use crate::system::{Context, EventForwarderActor, EventForwarderConsumer, SystemController};
use crate::territory::Territory;
use crate::traffic::Traffic;
use crate::traits::WithClock;
use crate::visited::Visited;
use crate::world::World;
use commons::process::Process;

pub struct System {
    cx: Context,
    rx: FnReceiver<Self>,
    run: bool,
    processes: Processes,
}

struct Processes {
    avatar_artist: Process<AvatarArtistActor<Context>>,
    avatar_visibility: Process<AvatarVisibility<Context>>,
    basic_avatar_controls: Process<BasicAvatarControls<Context>>,
    basic_road_builder: Process<BasicRoadBuilder<Context>>,
    builder: Process<BuilderActor<Context>>,
    cheats: Process<Cheats<Context>>,
    edge_sims: Vec<Process<EdgeBuildSimulation<Context, AutoRoadTravelDuration>>>,
    event_forwarder: Process<EventForwarderActor>,
    labels: Process<Labels<Context>>,
    object_builder: Process<ObjectBuilder<Context>>,
    pathfinding_avatar_controls: Process<PathfindingAvatarControls<Context>>,
    position_sims: Vec<Process<PositionBuildSimulation<Context>>>,
    prime_mover: Process<PrimeMover<Context>>,
    resource_targets: Process<ResourceTargets<Context>>,
    rotate: Process<Rotate<Context>>,
    settlement_sims: Vec<Process<SettlementSimulation<Context>>>,
    setup_new_world: Process<SetupNewWorld<Context>>,
    setup_pathfinders: Process<SetupPathfinders<Context>>,
    setup_visibility: Process<SetupVisibility<Context>>,
    speed_control: Process<SpeedControl<Context>>,
    town_builder: Process<TownBuilderActor<Context>>,
    town_house_artist: Process<TownHouseArtist<Context>>,
    town_label_artist: Process<TownLabelArtist<Context>>,
    voyager: Process<Voyager<Context>>,
    world_artist: Process<WorldArtistActor<Context>>,
    world_gen: Process<WorldGen<Context>>,
}

impl System {
    pub fn new(params: Parameters, engine: &mut IsometricEngine) -> System {
        let params = Arc::new(params);

        let avatar_travel_duration_with_planned_roads = Arc::new(
            AvatarTravelDuration::with_planned_roads_as_roads(&params.avatar_travel),
        );
        let avatar_travel_duration_without_planned_roads = Arc::new(
            AvatarTravelDuration::with_planned_roads_ignored(&params.avatar_travel),
        );
        let road_build_travel_duration = Arc::new(AutoRoadTravelDuration::from_params(
            &params.auto_road_travel,
        ));

        let (avatar_artist_tx, avatar_artist_rx) = fn_channel();
        let (avatar_visibility_tx, avatar_visibility_rx) = fn_channel();
        let (basic_avatar_controls_tx, basic_avatar_controls_rx) = fn_channel();
        let (basic_road_builder_tx, basic_road_builder_rx) = fn_channel();
        let (builder_tx, builder_rx) = fn_channel();
        let (cheats_tx, cheats_rx) = fn_channel();
        let (edge_sim_tx, edge_sim_rx) = fn_channel();
        let (labels_tx, labels_rx) = fn_channel();
        let (object_builder_tx, object_builder_rx) = fn_channel();
        let (pathfinding_avatar_controls_tx, pathfinding_avatar_controls_rx) = fn_channel();
        let (position_sim_tx, position_sim_rx) = fn_channel();
        let (prime_mover_tx, prime_mover_rx) = fn_channel();
        let (resource_targets_tx, resource_targets_rx) = fn_channel();
        let (rotate_tx, rotate_rx) = fn_channel();
        let (setup_new_world_tx, setup_new_world_rx) = fn_channel();
        let (setup_pathfinders_tx, setup_pathfinders_rx) = fn_channel();
        let (setup_visibility_tx, setup_visibility_rx) = fn_channel();
        let (speed_control_tx, speed_control_rx) = fn_channel();
        let (system_tx, system_rx) = fn_channel();
        let (town_builder_tx, town_builder_rx) = fn_channel();
        let (town_house_artist_tx, town_house_artist_rx) = fn_channel();
        let (town_label_artist_tx, town_label_artist_rx) = fn_channel();
        let (voyager_tx, voyager_rx) = fn_channel();
        let (world_artist_tx, world_artist_rx) = fn_channel();
        let (world_gen_tx, world_gen_rx) = fn_channel();

        let settlement_sim_channels = (0..params.simulation.threads)
            .map(|_| fn_channel())
            .collect::<Vec<_>>();
        let mut settlement_sim_txs = vec![];
        let mut settlement_sim_rxs = vec![];
        for (cx, rx) in settlement_sim_channels {
            settlement_sim_txs.push(cx);
            settlement_sim_rxs.push(rx);
        }

        let pool = ThreadPool::builder().name_prefix("pool").create().unwrap();

        let cx = Context {
            avatar_artist_tx,
            avatar_visibility_tx,
            avatars: Arc::default(),
            background_service: Arc::new(BackgroundService::new(pool.clone())),
            basic_avatar_controls_tx,
            basic_road_builder_tx,
            builder_tx,
            build_queue: Arc::default(),
            cheats_tx,
            clock: Arc::new(RwLock::new(Clock::new(RealTime {}, params.default_speed))),
            edge_sim_tx,
            edge_traffic: Arc::default(),
            engine_tx: engine.command_tx(),
            labels_tx,
            nations: Arc::default(),
            object_builder_tx,
            parameters: params.clone(),
            pathfinder_with_planned_roads: Arc::new(RwLock::new(Pathfinder::new(
                params.width,
                params.width,
                avatar_travel_duration_with_planned_roads,
            ))),
            pathfinder_without_planned_roads: Arc::new(RwLock::new(Pathfinder::new(
                params.width,
                params.width,
                avatar_travel_duration_without_planned_roads.clone(),
            ))),
            pathfinding_avatar_controls_tx,
            pool,
            position_sim_tx,
            prime_mover_tx,
            resource_targets_tx,
            rotate_tx,
            route_to_ports: Arc::default(),
            routes: Arc::default(),
            settlement_sim_txs,
            settlements: Arc::default(),
            setup_new_world_tx,
            setup_pathfinders_tx,
            setup_visibility_tx,
            sim_queue: Arc::default(),
            speed_control_tx,
            system_tx,
            territory: Arc::new(RwLock::new(Territory::new(params.width, params.width))),
            town_builder_tx,
            town_house_artist_tx,
            town_label_artist_tx,
            traffic: Arc::new(RwLock::new(Traffic::new(
                params.width,
                params.width,
                HashSet::with_capacity(0),
            ))),
            visibility: Arc::new(RwLock::new(VisibilityService::new())),
            visited: Arc::new(RwLock::new(Visited {
                visited: M::from_element(params.width, params.width, false),
                all_visited: params.reveal_all,
            })),
            voyager_tx,
            world: Arc::new(RwLock::new(World::new(M::zeros(1, 1), 0.0))),
            world_artist_tx,
            world_gen_tx,
        };

        let (event_forwarder_tx, event_forwarder_rx) = fn_channel();
        engine.add_event_consumer(EventForwarderConsumer::new(event_forwarder_tx));
        engine.add_event_consumer(SystemController::new(
            cx.clone_with_name("system_controller"),
        ));
        engine.add_event_handler(ZoomHandler::default());

        let system = System {
            cx: cx.clone_with_name("processes"),
            rx: system_rx,
            run: true,
            processes: Processes {
                avatar_artist: Process::new(
                    AvatarArtistActor::new(
                        cx.clone_with_name("avatar_artist"),
                        AvatarArtist::new(AvatarArtistParams::new(&params.light_direction)),
                    ),
                    avatar_artist_rx,
                ),
                avatar_visibility: Process::new(
                    AvatarVisibility::new(cx.clone_with_name("avatar_visibility")),
                    avatar_visibility_rx,
                ),
                basic_avatar_controls: Process::new(
                    BasicAvatarControls::new(
                        cx.clone_with_name("basic_avatar_controls"),
                        avatar_travel_duration_without_planned_roads.clone(),
                    ),
                    basic_avatar_controls_rx,
                ),
                basic_road_builder: Process::new(
                    BasicRoadBuilder::new(
                        cx.clone_with_name("basic_road_builder"),
                        avatar_travel_duration_without_planned_roads.clone(),
                        road_build_travel_duration.clone(),
                    ),
                    basic_road_builder_rx,
                ),
                builder: Process::new(
                    BuilderActor::new(
                        cx.clone_with_name("builder"),
                        vec![
                            Box::new(TownBuilder::new(cx.clone_with_name("town_builder"))),
                            Box::new(RoadBuilder::new(cx.clone_with_name("road_builder"))),
                            Box::new(CropsBuilder::new(cx.clone_with_name("crops_builder"))),
                        ],
                    ),
                    builder_rx,
                ),
                cheats: Process::new(Cheats::new(cx.clone_with_name("cheats")), cheats_rx),
                edge_sims: (0..params.simulation.threads)
                    .map(|_| {
                        Process::new(
                            EdgeBuildSimulation::new(
                                cx.clone_with_name("edge_sim"),
                                road_build_travel_duration.clone(),
                            ),
                            edge_sim_rx.clone(),
                        )
                    })
                    .collect(),
                event_forwarder: Process::new(
                    EventForwarderActor::new(cx.clone_with_name("event_forwarder")),
                    event_forwarder_rx,
                ),
                labels: Process::new(Labels::new(cx.clone_with_name("labels")), labels_rx),
                object_builder: Process::new(
                    ObjectBuilder::new(cx.clone_with_name("object_builder"), params.seed),
                    object_builder_rx,
                ),
                pathfinding_avatar_controls: Process::new(
                    PathfindingAvatarControls::new(
                        cx.clone_with_name("pathfinding_avatar_controls"),
                        avatar_travel_duration_without_planned_roads.clone(),
                    ),
                    pathfinding_avatar_controls_rx,
                ),
                position_sims: (0..params.simulation.threads)
                    .map(|_| {
                        Process::new(
                            PositionBuildSimulation::new(cx.clone_with_name("position_sim"), 0),
                            position_sim_rx.clone(),
                        )
                    })
                    .collect(),
                prime_mover: Process::new(
                    PrimeMover::new(
                        cx.clone_with_name("prime_mover"),
                        params.avatars,
                        params.seed,
                        avatar_travel_duration_without_planned_roads,
                        &params.nations,
                    ),
                    prime_mover_rx,
                ),
                resource_targets: Process::new(
                    ResourceTargets::new(cx.clone_with_name("resource_targets")),
                    resource_targets_rx,
                ),
                rotate: Process::new(Rotate::new(cx.clone_with_name("rotate")), rotate_rx),
                settlement_sims: settlement_sim_rxs
                    .into_iter()
                    .map(|rx| {
                        Process::new(
                            SettlementSimulation::new(cx.clone_with_name("settlement_simulation")),
                            rx,
                        )
                    })
                    .collect(),
                setup_new_world: Process::new(
                    SetupNewWorld::new(cx.clone_with_name("setup_new_world")),
                    setup_new_world_rx,
                ),
                setup_pathfinders: Process::new(
                    SetupPathfinders::new(cx.clone_with_name("setup_pathfinders")),
                    setup_pathfinders_rx,
                ),
                setup_visibility: Process::new(
                    SetupVisibility::new(cx.clone_with_name("setup_visibility")),
                    setup_visibility_rx,
                ),
                speed_control: Process::new(
                    SpeedControl::new(cx.clone_with_name("speed_control")),
                    speed_control_rx,
                ),
                town_builder: Process::new(
                    TownBuilderActor::new(cx.clone_with_name("town_builder_actor")),
                    town_builder_rx,
                ),
                town_house_artist: Process::new(
                    TownHouseArtist::new(cx.clone_with_name("town_houses"), params.town_artist),
                    town_house_artist_rx,
                ),
                town_label_artist: Process::new(
                    TownLabelArtist::new(cx.clone_with_name("town_labels"), params.town_artist),
                    town_label_artist_rx,
                ),
                voyager: Process::new(Voyager::new(cx.clone_with_name("voyager")), voyager_rx),
                world_artist: Process::new(
                    WorldArtistActor::new(
                        cx.clone_with_name("world_artist_actor"),
                        WorldArtist::new(
                            params.width,
                            params.width,
                            WorldArtistParameters {
                                waterfall_gradient: params
                                    .avatar_travel
                                    .max_navigable_river_gradient,
                                ..WorldArtistParameters::default()
                            },
                        ),
                        WorldColoringParameters {
                            colors: params.base_colors,
                            beach_level: params.world_gen.beach_level,
                            cliff_gradient: params.world_gen.cliff_gradient,
                            snow_temperature: params.snow_temperature,
                            light_direction: params.light_direction,
                        },
                        0.3,
                        &params.nations,
                    ),
                    world_artist_rx,
                ),
                world_gen: Process::new(
                    WorldGen::new(cx.clone_with_name("world_gen")),
                    world_gen_rx,
                ),
            },
        };

        system.send_init_messages();

        system
    }

    pub fn send_init_messages(&self) {
        self.cx
            .avatar_artist_tx
            .send_future(|avatar_artist| avatar_artist.init().boxed());
        self.cx
            .labels_tx
            .send_future(|labels| labels.init().boxed());
        self.cx
            .setup_visibility_tx
            .send_future(|setup_visibility| setup_visibility.init().boxed());
        self.cx
            .setup_pathfinders_tx
            .send_future(|setup_pathfinders| setup_pathfinders.init().boxed());
        self.cx
            .resource_targets_tx
            .send_future(|resource_targets| resource_targets.init().boxed());
        self.cx
            .system_tx
            .send_future(|system| system.start().boxed());
        self.cx
            .town_house_artist_tx
            .send_future(|town_house_artist| town_house_artist.init().boxed());
        self.cx
            .town_label_artist_tx
            .send_future(|town_label_artist| town_label_artist.init().boxed());
        self.cx
            .world_artist_tx
            .send_future(|world_artist| world_artist.init().boxed());
    }

    pub fn new_game(&self) {
        self.cx
            .prime_mover_tx
            .send_future(|prime_mover| prime_mover.new_game().boxed());
        self.cx
            .setup_new_world_tx
            .send_future(|setup_new_world| setup_new_world.new_game().boxed());
        self.cx
            .world_gen_tx
            .send_future(|world_gen| world_gen.new_game().boxed());
    }

    pub async fn start(&mut self) {
        self.processes.start(&self.cx.pool).await;

        self.cx.mut_clock(|clock| clock.resume()).await;
    }

    pub async fn pause(&mut self) {
        self.cx.mut_clock(|clock| clock.pause()).await;

        self.cx.background_service.wait_on_tasks();

        self.processes.pause(&self.cx.pool).await;
    }

    pub async fn save(&mut self, path: &str) {
        self.processes.save(path).await;

        self.cx.clock.write().await.save(&format!("{}.clock", path));

        self.cx
            .avatars
            .read()
            .await
            .save(&format!("{}.avatars", path));
        self.cx
            .build_queue
            .read()
            .await
            .save(&format!("{}.build_queue", path));
        self.cx
            .edge_traffic
            .read()
            .await
            .save(&format!("{}.edge_traffic", path));
        self.cx
            .nations
            .read()
            .await
            .save(&format!("{}.nations", path));
        self.cx.parameters.save(&format!("{}.parameters", path));
        self.cx
            .route_to_ports
            .read()
            .await
            .save(&format!("{}.route_to_ports", path));
        self.cx
            .routes
            .read()
            .await
            .save(&format!("{}.routes", path));
        self.cx
            .settlements
            .read()
            .await
            .save(&format!("{}.settlements", path));
        self.cx
            .sim_queue
            .read()
            .await
            .save(&format!("{}.sim_queue", path));
        self.cx
            .territory
            .read()
            .await
            .save(&format!("{}.territory", path));
        self.cx
            .traffic
            .read()
            .await
            .save(&format!("{}.traffic", path));
        self.cx
            .visited
            .read()
            .await
            .save(&format!("{}.visited", path));
        self.cx.world.read().await.save(&format!("{}.world", path));
    }

    pub async fn load(&mut self, path: &str) {
        self.processes.load(path).await;

        self.cx.clock.write().await.load(&format!("{}.clock", path));

        *self.cx.avatars.write().await = <_>::load(&format!("{}.avatars", path));
        *self.cx.build_queue.write().await = <_>::load(&format!("{}.build_queue", path));
        *self.cx.edge_traffic.write().await = <_>::load(&format!("{}.edge_traffic", path));
        *self.cx.nations.write().await = <_>::load(&format!("{}.nations", path));
        *self.cx.route_to_ports.write().await = <_>::load(&format!("{}.route_to_ports", path));
        *self.cx.routes.write().await = <_>::load(&format!("{}.routes", path));
        *self.cx.settlements.write().await = <_>::load(&format!("{}.settlements", path));
        *self.cx.sim_queue.write().await = <_>::load(&format!("{}.sim_queue", path));
        *self.cx.territory.write().await = <_>::load(&format!("{}.territory", path));
        *self.cx.traffic.write().await = <_>::load(&format!("{}.traffic", path));
        *self.cx.visited.write().await = <_>::load(&format!("{}.visited", path));
        *self.cx.world.write().await = <_>::load(&format!("{}.world", path));
    }

    pub fn run(mut self) -> RemoteHandle<()> {
        let pool = self.cx.pool.clone();
        let (runnable, handle) = async move {
            while self.run {
                self.rx.get_message().await.apply(&mut self).await;
            }
        }
        .remote_handle();
        pool.spawn_ok(runnable);
        handle
    }

    pub fn shutdown(&mut self) {
        self.run = false;
    }
}

impl Processes {
    async fn start(&mut self, pool: &ThreadPool) {
        self.world_gen.run_passive(pool).await;
        self.setup_visibility.run_passive(pool).await;
        self.setup_new_world.run_passive(pool).await;
        self.setup_pathfinders.run_passive(pool).await;

        self.world_artist.run_passive(pool).await;
        join_all(
            self.position_sims
                .iter_mut()
                .map(|sim| sim.run_passive(pool)),
        )
        .await;
        self.voyager.run_passive(pool).await;
        self.town_house_artist.run_passive(pool).await;
        self.town_label_artist.run_passive(pool).await;
        join_all(self.edge_sims.iter_mut().map(|sim| sim.run_passive(pool))).await;
        self.resource_targets.run_passive(pool).await;
        self.rotate.run_passive(pool).await;
        self.town_builder.run_passive(pool).await;
        self.speed_control.run_passive(pool).await;
        join_all(
            self.settlement_sims
                .iter_mut()
                .map(|sim| sim.run_active(pool)),
        )
        .await;
        self.prime_mover.run_active(pool).await;
        self.pathfinding_avatar_controls.run_passive(pool).await;
        self.object_builder.run_passive(pool).await;
        self.labels.run_passive(pool).await;
        self.cheats.run_passive(pool).await;
        self.builder.run_active(pool).await;
        self.basic_road_builder.run_passive(pool).await;
        self.basic_avatar_controls.run_passive(pool).await;
        self.avatar_visibility.run_active(pool).await;
        self.avatar_artist.run_passive(pool).await;
        self.event_forwarder.run_passive(pool).await;
    }

    async fn pause(&mut self, pool: &ThreadPool) {
        self.event_forwarder.drain(pool, false).await;
        self.avatar_artist.drain(pool, true).await;
        self.avatar_visibility.drain(pool, true).await;
        self.basic_avatar_controls.drain(pool, true).await;
        self.basic_road_builder.drain(pool, true).await;
        self.builder.drain(pool, true).await;
        self.cheats.drain(pool, true).await;
        self.labels.drain(pool, true).await;
        self.object_builder.drain(pool, true).await;
        self.pathfinding_avatar_controls.drain(pool, true).await;
        self.prime_mover.drain(pool, true).await;
        join_all(
            self.settlement_sims
                .iter_mut()
                .map(|sim| sim.drain(pool, true)),
        )
        .await;
        self.speed_control.drain(pool, true).await;
        self.town_builder.drain(pool, true).await;
        self.rotate.drain(pool, true).await;
        self.resource_targets.drain(pool, true).await;
        join_all(self.edge_sims.iter_mut().map(|sim| sim.drain(pool, true))).await;
        self.town_label_artist.drain(pool, true).await;
        self.town_house_artist.drain(pool, true).await;
        self.voyager.drain(pool, true).await;
        join_all(
            self.position_sims
                .iter_mut()
                .map(|sim| sim.drain(pool, true)),
        )
        .await;
        self.world_artist.drain(pool, true).await;

        self.setup_pathfinders.drain(pool, true).await;
        self.setup_new_world.drain(pool, true).await;
        self.setup_visibility.drain(pool, true).await;
        self.world_gen.drain(pool, true).await;
    }

    async fn save(&mut self, path: &str) {
        self.labels.object_ref().unwrap().save(path);
        self.prime_mover.object_ref().unwrap().save(path);
    }

    async fn load(&mut self, path: &str) {
        self.labels.object_mut().unwrap().load(path);
        self.prime_mover.object_mut().unwrap().load(path);
    }
}
