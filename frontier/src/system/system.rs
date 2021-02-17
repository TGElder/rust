use std::collections::HashSet;
use std::sync::Arc;

use commons::async_std::sync::RwLock;
use commons::fn_sender::fn_channel;
use commons::persistence::{Load, Save};
use commons::M;
use futures::executor::ThreadPool;
use futures::future::FutureExt;
use isometric::event_handlers::ZoomHandler;
use isometric::IsometricEngine;

use crate::actors::{
    AvatarArtistActor, AvatarVisibility, BasicAvatarControls, BasicRoadBuilder, BuilderActor,
    Cheats, Clock, Labels, ObjectBuilder, PathfindingAvatarControls, PrimeMover, RealTime,
    ResourceTargets, Rotate, SetupNewWorld, SetupPathfinders, SpeedControl, TownBuilderActor,
    TownHouseArtist, TownLabelArtist, VisibilityActor, Voyager, WorldArtistActor,
    WorldColoringParameters, WorldGen,
};
use crate::artists::{AvatarArtist, AvatarArtistParams, WorldArtist, WorldArtistParameters};
use crate::avatar::AvatarTravelDuration;
use crate::build::builders::{CropsBuilder, RoadBuilder, TownBuilder};
use crate::parameters::Parameters;
use crate::pathfinder::Pathfinder;
use crate::road_builder::AutoRoadTravelDuration;
use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::simulation::settlement::SettlementSimulation;
use crate::system::{EventForwarderActor, EventForwarderConsumer, Polysender};
use crate::territory::Territory;
use crate::traffic::Traffic;
use crate::traits::WithClock;
use crate::world::World;
use commons::process::Process;

pub struct System {
    pub tx: Polysender,
    pub avatar_artist: Process<AvatarArtistActor<Polysender>>,
    pub avatar_visibility: Process<AvatarVisibility<Polysender>>,
    pub basic_avatar_controls: Process<BasicAvatarControls<Polysender>>,
    pub basic_road_builder: Process<BasicRoadBuilder<Polysender>>,
    pub builder: Process<BuilderActor<Polysender>>,
    pub cheats: Process<Cheats<Polysender>>,
    pub edge_sim: Process<EdgeBuildSimulation<Polysender, AutoRoadTravelDuration>>,
    pub event_forwarder: Process<EventForwarderActor>,
    pub labels: Process<Labels<Polysender>>,
    pub object_builder: Process<ObjectBuilder<Polysender>>,
    pub pathfinding_avatar_controls: Process<PathfindingAvatarControls<Polysender>>,
    pub position_sim: Process<PositionBuildSimulation<Polysender>>,
    pub prime_mover: Process<PrimeMover<Polysender>>,
    pub resource_targets: Process<ResourceTargets<Polysender>>,
    pub rotate: Process<Rotate>,
    pub settlement_sims: Vec<Process<SettlementSimulation<Polysender>>>,
    pub setup_new_world: Process<SetupNewWorld<Polysender>>,
    pub setup_pathfinders: Process<SetupPathfinders<Polysender>>,
    pub speed_control: Process<SpeedControl<Polysender>>,
    pub town_builder: Process<TownBuilderActor<Polysender>>,
    pub town_house_artist: Process<TownHouseArtist<Polysender>>,
    pub town_label_artist: Process<TownLabelArtist<Polysender>>,
    pub visibility: Process<VisibilityActor<Polysender>>,
    pub voyager: Process<Voyager<Polysender>>,
    pub world_artist: Process<WorldArtistActor<Polysender>>,
    pub world_gen: Process<WorldGen<Polysender>>,
}

impl System {
    pub fn new(params: Parameters, engine: &mut IsometricEngine, pool: ThreadPool) -> System {
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
        let (speed_control_tx, speed_control_rx) = fn_channel();
        let (town_builder_tx, town_builder_rx) = fn_channel();
        let (town_house_artist_tx, town_house_artist_rx) = fn_channel();
        let (town_label_artist_tx, town_label_artist_rx) = fn_channel();
        let (visibility_tx, visibility_rx) = fn_channel();
        let (voyager_tx, voyager_rx) = fn_channel();
        let (world_artist_tx, world_artist_rx) = fn_channel();
        let (world_gen_tx, world_gen_rx) = fn_channel();

        let settlement_sim_channels = (0..params.simulation.threads)
            .map(|_| fn_channel())
            .collect::<Vec<_>>();
        let mut settlement_sim_txs = vec![];
        let mut settlement_sim_rxs = vec![];
        for (tx, rx) in settlement_sim_channels {
            settlement_sim_txs.push(tx);
            settlement_sim_rxs.push(rx);
        }

        let tx = Polysender {
            avatar_artist_tx,
            avatar_visibility_tx,
            avatars: Arc::default(),
            basic_avatar_controls_tx,
            basic_road_builder_tx,
            builder_tx,
            build_queue: Arc::default(),
            cheats_tx,
            clock: Arc::new(RwLock::new(Clock::new(RealTime {}, params.default_speed))),
            edge_sim_tx,
            edge_traffic: Arc::default(),
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
            routes: Arc::default(),
            rotate_tx,
            route_to_ports: Arc::default(),
            settlement_sim_txs,
            settlements: Arc::default(),
            setup_pathfinders_tx,
            setup_new_world_tx,
            sim_queue: Arc::default(),
            speed_control_tx,
            territory: Arc::new(RwLock::new(Territory::new(params.width, params.width))),
            traffic: Arc::new(RwLock::new(Traffic::new(
                params.width,
                params.width,
                HashSet::with_capacity(0),
            ))),
            town_builder_tx,
            town_house_artist_tx,
            town_label_artist_tx,
            visibility_tx,
            voyager_tx,
            world: Arc::new(RwLock::new(World::new(M::zeros(1, 1), 0.0))),
            world_artist_tx,
            world_gen_tx,
        };

        let (event_forwarder_tx, event_forwarder_rx) = fn_channel();
        engine.add_event_consumer(EventForwarderConsumer::new(event_forwarder_tx));
        engine.add_event_handler(ZoomHandler::default());

        let config = System {
            tx: tx.clone_with_name("processes"),
            avatar_artist: Process::new(
                AvatarArtistActor::new(
                    tx.clone_with_name("avatar_artist"),
                    engine.command_tx(),
                    AvatarArtist::new(AvatarArtistParams::new(&params.light_direction)),
                ),
                avatar_artist_rx,
            ),
            avatar_visibility: Process::new(
                AvatarVisibility::new(tx.clone_with_name("avatar_visibility")),
                avatar_visibility_rx,
            ),
            basic_avatar_controls: Process::new(
                BasicAvatarControls::new(
                    tx.clone_with_name("basic_avatar_controls"),
                    avatar_travel_duration_without_planned_roads.clone(),
                ),
                basic_avatar_controls_rx,
            ),
            basic_road_builder: Process::new(
                BasicRoadBuilder::new(
                    tx.clone_with_name("basic_road_builder"),
                    avatar_travel_duration_without_planned_roads.clone(),
                    road_build_travel_duration.clone(),
                ),
                basic_road_builder_rx,
            ),
            builder: Process::new(
                BuilderActor::new(
                    tx.clone_with_name("builder"),
                    vec![
                        Box::new(TownBuilder::new(tx.clone_with_name("town_builder"))),
                        Box::new(RoadBuilder::new(tx.clone_with_name("road_builder"))),
                        Box::new(CropsBuilder::new(tx.clone_with_name("crops_builder"))),
                    ],
                ),
                builder_rx,
            ),
            cheats: Process::new(Cheats::new(tx.clone_with_name("cheats")), cheats_rx),
            edge_sim: Process::new(
                EdgeBuildSimulation::new(
                    tx.clone_with_name("edge_sim"),
                    road_build_travel_duration,
                ),
                edge_sim_rx,
            ),
            event_forwarder: Process::new(
                EventForwarderActor::new(tx.clone_with_name("event_forwarder")),
                event_forwarder_rx,
            ),
            labels: Process::new(
                Labels::new(tx.clone_with_name("labels"), engine.command_tx()),
                labels_rx,
            ),
            object_builder: Process::new(
                ObjectBuilder::new(tx.clone_with_name("object_builder"), params.seed),
                object_builder_rx,
            ),
            pathfinding_avatar_controls: Process::new(
                PathfindingAvatarControls::new(
                    tx.clone_with_name("pathfinding_avatar_controls"),
                    avatar_travel_duration_without_planned_roads.clone(),
                ),
                pathfinding_avatar_controls_rx,
            ),
            position_sim: Process::new(
                PositionBuildSimulation::new(tx.clone_with_name("position_sim"), 0),
                position_sim_rx,
            ),
            prime_mover: Process::new(
                PrimeMover::new(
                    tx.clone_with_name("prime_mover"),
                    params.avatars,
                    params.seed,
                    avatar_travel_duration_without_planned_roads,
                    &params.nations,
                ),
                prime_mover_rx,
            ),
            resource_targets: Process::new(
                ResourceTargets::new(tx.clone_with_name("resource_targets")),
                resource_targets_rx,
            ),
            rotate: Process::new(Rotate::new(engine.command_tx()), rotate_rx),
            settlement_sims: settlement_sim_rxs
                .into_iter()
                .map(|rx| {
                    Process::new(
                        SettlementSimulation::new(tx.clone_with_name("settlement_simulation")),
                        rx,
                    )
                })
                .collect(),
            setup_new_world: Process::new(
                SetupNewWorld::new(tx.clone_with_name("setup_new_world")),
                setup_new_world_rx,
            ),
            setup_pathfinders: Process::new(
                SetupPathfinders::new(tx.clone_with_name("setup_pathfinders")),
                setup_pathfinders_rx,
            ),
            speed_control: Process::new(
                SpeedControl::new(tx.clone_with_name("speed_control")),
                speed_control_rx,
            ),
            town_builder: Process::new(
                TownBuilderActor::new(tx.clone_with_name("town_builder_actor")),
                town_builder_rx,
            ),
            town_house_artist: Process::new(
                TownHouseArtist::new(
                    tx.clone_with_name("town_houses"),
                    engine.command_tx(),
                    params.town_artist,
                ),
                town_house_artist_rx,
            ),
            town_label_artist: Process::new(
                TownLabelArtist::new(
                    tx.clone_with_name("town_labels"),
                    engine.command_tx(),
                    params.town_artist,
                ),
                town_label_artist_rx,
            ),
            visibility: Process::new(
                VisibilityActor::new(tx.clone_with_name("visibility")),
                visibility_rx,
            ),
            voyager: Process::new(Voyager::new(tx.clone_with_name("voyager")), voyager_rx),
            world_artist: Process::new(
                WorldArtistActor::new(
                    tx.clone_with_name("world_artist_actor"),
                    engine.command_tx(),
                    WorldArtist::new(
                        params.width,
                        params.width,
                        WorldArtistParameters {
                            waterfall_gradient: params.avatar_travel.max_navigable_river_gradient,
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
            world_gen: Process::new(WorldGen::new(tx.clone_with_name("world_gen")), world_gen_rx),
        };

        config.send_init_messages();

        config
    }

    pub fn send_init_messages(&self) {
        self.tx
            .avatar_artist_tx
            .send_future(|avatar_artist| avatar_artist.init().boxed());
        self.tx
            .labels_tx
            .send_future(|labels| labels.init().boxed());
        self.tx
            .setup_pathfinders_tx
            .send_future(|setup_pathfinders| setup_pathfinders.init().boxed());
        self.tx
            .resource_targets_tx
            .send_future(|resource_targets| resource_targets.init().boxed());
        self.tx
            .town_house_artist_tx
            .send_future(|town_house_artist| town_house_artist.init().boxed());
        self.tx
            .town_label_artist_tx
            .send_future(|town_label_artist| town_label_artist.init().boxed());
        self.tx
            .visibility_tx
            .send_future(|visibility| visibility.init().boxed());
        self.tx
            .world_artist_tx
            .send_future(|world_artist| world_artist.init().boxed());
    }

    pub fn new_game(&self) {
        self.tx
            .prime_mover_tx
            .send_future(|prime_mover| prime_mover.new_game().boxed());
        self.tx
            .setup_new_world_tx
            .send_future(|setup_new_world| setup_new_world.new_game().boxed());
        self.tx
            .visibility_tx
            .send_future(|visibility| visibility.new_game().boxed());
        self.tx
            .world_gen_tx
            .send_future(|world_gen| world_gen.new_game().boxed());
    }

    pub async fn start(&mut self) {
        self.world_gen.run_passive(&self.tx.pool).await;
        self.setup_new_world.run_passive(&self.tx.pool).await;
        self.setup_pathfinders.run_passive(&self.tx.pool).await;

        self.world_artist.run_passive(&self.tx.pool).await;
        self.position_sim.run_passive(&self.tx.pool).await;
        self.voyager.run_passive(&self.tx.pool).await;
        self.visibility.run_passive(&self.tx.pool).await;
        self.town_house_artist.run_passive(&self.tx.pool).await;
        self.town_label_artist.run_passive(&self.tx.pool).await;
        self.edge_sim.run_passive(&self.tx.pool).await;
        self.resource_targets.run_passive(&self.tx.pool).await;
        self.rotate.run_passive(&self.tx.pool).await;
        self.town_builder.run_passive(&self.tx.pool).await;
        self.speed_control.run_passive(&self.tx.pool).await;
        for settlement_sim in &mut self.settlement_sims {
            settlement_sim.run_active(&self.tx.pool).await;
        }
        self.prime_mover.run_active(&self.tx.pool).await;
        self.pathfinding_avatar_controls
            .run_passive(&self.tx.pool)
            .await;
        self.object_builder.run_passive(&self.tx.pool).await;
        self.labels.run_passive(&self.tx.pool).await;
        self.cheats.run_passive(&self.tx.pool).await;
        self.builder.run_active(&self.tx.pool).await;
        self.basic_road_builder.run_passive(&self.tx.pool).await;
        self.basic_avatar_controls.run_passive(&self.tx.pool).await;
        self.avatar_visibility.run_active(&self.tx.pool).await;
        self.avatar_artist.run_passive(&self.tx.pool).await;
        self.event_forwarder.run_passive(&self.tx.pool).await;

        self.tx.mut_clock(|clock| clock.resume()).await;
    }

    pub async fn pause(&mut self) {
        self.tx.mut_clock(|clock| clock.pause()).await;

        self.event_forwarder.drain(&self.tx.pool, false).await;
        self.avatar_artist.drain(&self.tx.pool, true).await;
        self.avatar_visibility.drain(&self.tx.pool, true).await;
        self.basic_avatar_controls.drain(&self.tx.pool, true).await;
        self.basic_road_builder.drain(&self.tx.pool, true).await;
        self.builder.drain(&self.tx.pool, true).await;
        self.cheats.drain(&self.tx.pool, true).await;
        self.labels.drain(&self.tx.pool, true).await;
        self.object_builder.drain(&self.tx.pool, true).await;
        self.pathfinding_avatar_controls
            .drain(&self.tx.pool, true)
            .await;
        self.prime_mover.drain(&self.tx.pool, true).await;
        for settlement_sim in &mut self.settlement_sims {
            settlement_sim.drain(&self.tx.pool, true).await;
        }
        self.speed_control.drain(&self.tx.pool, true).await;
        self.town_builder.drain(&self.tx.pool, true).await;
        self.rotate.drain(&self.tx.pool, true).await;
        self.resource_targets.drain(&self.tx.pool, true).await;
        self.edge_sim.drain(&self.tx.pool, true).await;
        self.town_label_artist.drain(&self.tx.pool, true).await;
        self.town_house_artist.drain(&self.tx.pool, true).await;
        self.visibility.drain(&self.tx.pool, true).await;
        self.voyager.drain(&self.tx.pool, true).await;
        self.position_sim.drain(&self.tx.pool, true).await;
        self.world_artist.drain(&self.tx.pool, true).await;

        self.setup_pathfinders.drain(&self.tx.pool, true).await;
        self.setup_new_world.drain(&self.tx.pool, true).await;
        self.world_gen.drain(&self.tx.pool, true).await;
    }

    pub async fn save(&mut self, path: &str) {
        self.labels.object_ref().unwrap().save(path);
        self.prime_mover.object_ref().unwrap().save(path);
        self.visibility.object_ref().unwrap().save(path);

        self.tx.clock.write().await.save(&format!("{}.clock", path));

        self.tx
            .avatars
            .read()
            .await
            .save(&format!("{}.avatars", path));
        self.tx
            .build_queue
            .read()
            .await
            .save(&format!("{}.build_queue", path));
        self.tx
            .edge_traffic
            .read()
            .await
            .save(&format!("{}.edge_traffic", path));
        self.tx
            .nations
            .read()
            .await
            .save(&format!("{}.nations", path));
        self.tx.parameters.save(&format!("{}.parameters", path));
        self.tx
            .route_to_ports
            .read()
            .await
            .save(&format!("{}.route_to_ports", path));
        self.tx
            .routes
            .read()
            .await
            .save(&format!("{}.routes", path));
        self.tx
            .settlements
            .read()
            .await
            .save(&format!("{}.settlements", path));
        self.tx
            .sim_queue
            .read()
            .await
            .save(&format!("{}.sim_queue", path));
        self.tx
            .territory
            .read()
            .await
            .save(&format!("{}.territory", path));
        self.tx
            .traffic
            .read()
            .await
            .save(&format!("{}.traffic", path));
        self.tx.world.read().await.save(&format!("{}.world", path));
    }

    pub async fn load(&mut self, path: &str) {
        self.labels.object_mut().unwrap().load(path);
        self.prime_mover.object_mut().unwrap().load(path);
        self.visibility.object_mut().unwrap().load(path);

        self.tx.clock.write().await.load(&format!("{}.clock", path));

        *self.tx.avatars.write().await = <_>::load(&format!("{}.avatars", path));
        *self.tx.build_queue.write().await = <_>::load(&format!("{}.build_queue", path));
        *self.tx.edge_traffic.write().await = <_>::load(&format!("{}.edge_traffic", path));
        *self.tx.nations.write().await = <_>::load(&format!("{}.nations", path));
        *self.tx.route_to_ports.write().await = <_>::load(&format!("{}.route_to_ports", path));
        *self.tx.routes.write().await = <_>::load(&format!("{}.routes", path));
        *self.tx.settlements.write().await = <_>::load(&format!("{}.settlements", path));
        *self.tx.sim_queue.write().await = <_>::load(&format!("{}.sim_queue", path));
        *self.tx.territory.write().await = <_>::load(&format!("{}.territory", path));
        *self.tx.traffic.write().await = <_>::load(&format!("{}.traffic", path));
        *self.tx.world.write().await = <_>::load(&format!("{}.world", path));
    }
}
