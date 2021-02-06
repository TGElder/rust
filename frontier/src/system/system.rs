use std::collections::HashSet;
use std::sync::Arc;

use commons::async_std::sync::RwLock;
use commons::fn_sender::fn_channel;
use commons::persistence::{Load, Save};
use futures::executor::ThreadPool;
use futures::future::FutureExt;
use isometric::event_handlers::ZoomHandler;
use isometric::IsometricEngine;

use crate::actors::{
    AvatarArtistActor, AvatarVisibility, AvatarsActor, BasicAvatarControls, BasicRoadBuilder,
    BuilderActor, Cheats, Clock, Labels, Nations, ObjectBuilder, PathfinderService,
    PathfindingAvatarControls, PrimeMover, RealTime, ResourceTargets, Rotate, RoutesActor,
    Settlements, SetupNewWorld, SpeedControl, TerritoryActor, TownBuilderActor, TownHouseArtist,
    TownLabelArtist, VisibilityActor, Voyager, WorldActor, WorldArtistActor,
    WorldColoringParameters,
};
use crate::artists::{AvatarArtist, AvatarArtistParams, WorldArtist, WorldArtistParameters};
use crate::avatar::{AvatarTravelDuration, AvatarTravelModeFn};
use crate::build::builders::{CropsBuilder, RoadBuilder, TownBuilder};
use crate::build_sim::processors::{BuildCrops, BuildRoad, BuildTown, RemoveCrops, RemoveRoad};
use crate::build_sim::BuildSimulation;
use crate::parameters::Parameters;
use crate::pathfinder::Pathfinder;
use crate::road_builder::AutoRoadTravelDuration;
use crate::simulation::demand_fn::{homeland_demand_fn, town_demand_fn};
use crate::simulation::processors::{
    max_abs_population_change, GetDemand, GetRouteChanges, GetRoutes, GetTerritory, GetTownTraffic,
    InstructionLogger, RemoveTown, StepHomeland, StepTown, UpdateCurrentPopulation,
    UpdateEdgeTraffic, UpdateHomelandPopulation, UpdatePositionTraffic, UpdateRouteToPorts,
    UpdateTown,
};
use crate::simulation::Simulation;
use crate::system::{EventForwarderActor, EventForwarderConsumer, Polysender};
use crate::traffic::Traffic;
use crate::traits::SendClock;
use commons::process::Process;

pub struct System {
    pub tx: Polysender,
    pub pool: ThreadPool,
    pub avatars: Process<AvatarsActor>,
    pub avatar_artist: Process<AvatarArtistActor<Polysender>>,
    pub avatar_visibility: Process<AvatarVisibility<Polysender>>,
    pub basic_avatar_controls: Process<BasicAvatarControls<Polysender>>,
    pub basic_road_builder: Process<BasicRoadBuilder<Polysender>>,
    pub build_sim: Process<BuildSimulation>,
    pub builder: Process<BuilderActor<Polysender>>,
    pub cheats: Process<Cheats<Polysender>>,
    pub clock: Process<Clock<RealTime>>,
    pub event_forwarder: Process<EventForwarderActor>,
    pub labels: Process<Labels<Polysender>>,
    pub nations: Process<Nations>,
    pub object_builder: Process<ObjectBuilder<Polysender>>,
    pub parameters: Process<Parameters>,
    pub pathfinding_avatar_controls: Process<PathfindingAvatarControls<Polysender>>,
    pub pathfinder_with_planned_roads: Process<PathfinderService<Polysender, AvatarTravelDuration>>,
    pub pathfinder_without_planned_roads:
        Process<PathfinderService<Polysender, AvatarTravelDuration>>,
    pub prime_mover: Process<PrimeMover<Polysender>>,
    pub resource_targets: Process<ResourceTargets<Polysender>>,
    pub rotate: Process<Rotate>,
    pub routes: Process<RoutesActor>,
    pub settlements: Process<Settlements>,
    pub setup_new_world: Process<SetupNewWorld<Polysender>>,
    pub simulation: Process<Simulation>,
    pub speed_control: Process<SpeedControl<Polysender>>,
    pub territory: Process<TerritoryActor>,
    pub town_builder: Process<TownBuilderActor<Polysender>>,
    pub town_house_artist: Process<TownHouseArtist<Polysender>>,
    pub town_label_artist: Process<TownLabelArtist<Polysender>>,
    pub visibility: Process<VisibilityActor<Polysender>>,
    pub voyager: Process<Voyager<Polysender>>,
    pub world: Process<WorldActor<Polysender>>,
    pub world_artist: Process<WorldArtistActor<Polysender>>,
}

impl System {
    pub fn new(params: Parameters, engine: &mut IsometricEngine, pool: ThreadPool) -> System {
        let (avatar_artist_tx, avatar_artist_rx) = fn_channel();
        let (avatar_visibility_tx, avatar_visibility_rx) = fn_channel();
        let (avatars_tx, avatars_rx) = fn_channel();
        let (basic_avatar_controls_tx, basic_avatar_controls_rx) = fn_channel();
        let (basic_road_builder_tx, basic_road_builder_rx) = fn_channel();
        let (build_sim_tx, build_sim_rx) = fn_channel();
        let (builder_tx, builder_rx) = fn_channel();
        let (cheats_tx, cheats_rx) = fn_channel();
        let (clock_tx, clock_rx) = fn_channel();
        let (labels_tx, labels_rx) = fn_channel();
        let (nations_tx, nations_rx) = fn_channel();
        let (object_builder_tx, object_builder_rx) = fn_channel();
        let (parameters_tx, parameters_rx) = fn_channel();
        let (pathfinder_with_planned_roads_tx, pathfinder_with_planned_roads_rx) = fn_channel();
        let (pathfinder_without_planned_roads_tx, pathfinder_without_planned_roads_rx) =
            fn_channel();
        let (pathfinding_avatar_controls_tx, pathfinding_avatar_controls_rx) = fn_channel();
        let (prime_mover_tx, prime_mover_rx) = fn_channel();
        let (resource_targets_tx, resource_targets_rx) = fn_channel();
        let (rotate_tx, rotate_rx) = fn_channel();
        let (routes_tx, routes_rx) = fn_channel();
        let (settlements_tx, settlements_rx) = fn_channel();
        let (setup_new_world_tx, setup_new_world_rx) = fn_channel();
        let (simulation_tx, simulation_rx) = fn_channel();
        let (speed_control_tx, speed_control_rx) = fn_channel();
        let (territory_tx, territory_rx) = fn_channel();
        let (town_builder_tx, town_builder_rx) = fn_channel();
        let (town_house_artist_tx, town_house_artist_rx) = fn_channel();
        let (town_label_artist_tx, town_label_artist_rx) = fn_channel();
        let (visibility_tx, visibility_rx) = fn_channel();
        let (voyager_tx, voyager_rx) = fn_channel();
        let (world_tx, world_rx) = fn_channel();
        let (world_artist_tx, world_artist_rx) = fn_channel();

        let tx = Polysender {
            avatar_artist_tx,
            avatar_visibility_tx,
            avatars_tx,
            basic_avatar_controls_tx,
            basic_road_builder_tx,
            build_sim_tx,
            builder_tx,
            build_queue: Arc::default(),
            cheats_tx,
            clock_tx,
            edge_traffic: Arc::default(),
            labels_tx,
            nations_tx,
            object_builder_tx,
            parameters_tx,
            pathfinder_with_planned_roads_tx,
            pathfinder_without_planned_roads_tx,
            pathfinding_avatar_controls_tx,
            prime_mover_tx,
            resource_targets_tx,
            routes_tx,
            setup_new_world_tx,
            traffic: Arc::new(RwLock::new(Traffic::new(
                params.width,
                params.width,
                HashSet::with_capacity(0),
            ))),
            rotate_tx,
            route_to_ports: Arc::default(),
            settlements_tx,
            simulation_tx,
            speed_control_tx,
            territory_tx,
            town_builder_tx,
            town_house_artist_tx,
            town_label_artist_tx,
            visibility_tx,
            voyager_tx,
            world_tx,
            world_artist_tx,
        };

        let (event_forwarder_tx, event_forwarder_rx) = fn_channel();
        engine.add_event_consumer(EventForwarderConsumer::new(event_forwarder_tx));
        engine.add_event_handler(ZoomHandler::default());

        let avatar_travel_duration_with_planned_roads = Arc::new(
            AvatarTravelDuration::with_planned_roads_as_roads(&params.avatar_travel),
        );
        let avatar_travel_duration_without_planned_roads = Arc::new(
            AvatarTravelDuration::with_planned_roads_ignored(&params.avatar_travel),
        );
        let road_build_travel_duration = Arc::new(AutoRoadTravelDuration::from_params(
            &params.auto_road_travel,
        ));

        let config = System {
            tx: tx.clone_with_name("processes"),
            pool,
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
            avatars: Process::new(AvatarsActor::new(), avatars_rx),
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
            build_sim: Process::new(
                BuildSimulation::new(vec![
                    Box::new(BuildTown::new(
                        tx.clone_with_name("build_town"),
                        params.simulation.initial_town_population,
                    )),
                    Box::new(BuildCrops::new(
                        tx.clone_with_name("build_crops"),
                        params.seed,
                    )),
                    Box::new(RemoveCrops::new(tx.clone_with_name("remove_crops"))),
                    Box::new(BuildRoad::new(
                        tx.clone_with_name("build_road"),
                        road_build_travel_duration,
                        params.simulation.road_build_threshold,
                    )),
                    Box::new(RemoveRoad::new(tx.clone_with_name("remove_road"))),
                ]),
                build_sim_rx,
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
            clock: Process::new(Clock::new(RealTime {}, params.default_speed), clock_rx),
            event_forwarder: Process::new(
                EventForwarderActor::new(tx.clone_with_name("event_forwarder")),
                event_forwarder_rx,
            ),
            labels: Process::new(
                Labels::new(tx.clone_with_name("labels"), engine.command_tx()),
                labels_rx,
            ),
            nations: Process::new(Nations::new(), nations_rx),
            object_builder: Process::new(
                ObjectBuilder::new(tx.clone_with_name("object_builder"), params.seed),
                object_builder_rx,
            ),
            pathfinder_with_planned_roads: Process::new(
                PathfinderService::new(
                    tx.clone_with_name("pathfinder_with_planned_roads"),
                    Pathfinder::new(
                        params.width,
                        params.width,
                        avatar_travel_duration_with_planned_roads,
                    ),
                ),
                pathfinder_with_planned_roads_rx,
            ),
            pathfinder_without_planned_roads: Process::new(
                PathfinderService::new(
                    tx.clone_with_name("pathfinder_without_planned_roads"),
                    Pathfinder::new(
                        params.width,
                        params.width,
                        avatar_travel_duration_without_planned_roads.clone(),
                    ),
                ),
                pathfinder_without_planned_roads_rx,
            ),
            pathfinding_avatar_controls: Process::new(
                PathfindingAvatarControls::new(
                    tx.clone_with_name("pathfinding_avatar_controls"),
                    avatar_travel_duration_without_planned_roads.clone(),
                ),
                pathfinding_avatar_controls_rx,
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
            routes: Process::new(RoutesActor::new(), routes_rx),
            settlements: Process::new(Settlements::new(), settlements_rx),
            setup_new_world: Process::new(
                SetupNewWorld::new(tx.clone_with_name("setup_new_world")),
                setup_new_world_rx,
            ),
            simulation: Process::new(
                Simulation::new(vec![
                    Box::new(InstructionLogger::new()),
                    Box::new(StepHomeland::new(tx.clone_with_name("step_homeland"))),
                    Box::new(StepTown::new(tx.clone_with_name("step_town"))),
                    Box::new(GetTerritory::new(tx.clone_with_name("get_territory"))),
                    Box::new(GetTownTraffic::new(tx.clone_with_name("get_town_traffic"))),
                    Box::new(UpdateTown::new(
                        tx.clone_with_name("update_town"),
                        params.simulation.traffic_to_population,
                        params.simulation.nation_flip_traffic_pc,
                    )),
                    Box::new(RemoveTown::new(
                        tx.clone_with_name("remove_town"),
                        params.simulation.town_removal_population,
                    )),
                    Box::new(UpdateHomelandPopulation::new(
                        tx.clone_with_name("update_homeland_population"),
                    )),
                    Box::new(UpdateCurrentPopulation::new(
                        tx.clone_with_name("update_current_population"),
                        max_abs_population_change,
                    )),
                    Box::new(GetDemand::new(town_demand_fn)),
                    Box::new(GetDemand::new(homeland_demand_fn)),
                    Box::new(GetRoutes::new(tx.clone_with_name("get_routes"))),
                    Box::new(GetRouteChanges::new(
                        tx.clone_with_name("get_route_changes"),
                    )),
                    Box::new(UpdatePositionTraffic::new(
                        tx.clone_with_name("update_position_traffic"),
                    )),
                    Box::new(UpdateRouteToPorts::new(
                        tx.clone_with_name("update_route_to_ports"),
                        Arc::new(AvatarTravelModeFn::new(
                            params.avatar_travel.min_navigable_river_width,
                        )),
                    )),
                    Box::new(UpdateEdgeTraffic::new(
                        tx.clone_with_name("update_edge_traffic"),
                    )),
                ]),
                simulation_rx,
            ),
            speed_control: Process::new(
                SpeedControl::new(tx.clone_with_name("speed_control")),
                speed_control_rx,
            ),
            territory: Process::new(
                TerritoryActor::new(params.width, params.width),
                territory_rx,
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
            world: Process::new(WorldActor::new(tx.clone_with_name("world")), world_rx),
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
            parameters: Process::new(params, parameters_rx),
        };

        config.send_init_messages();

        config
    }

    pub fn send_init_messages(&self) {
        self.tx
            .avatar_artist_tx
            .send(|avatar_artist| avatar_artist.init());
        self.tx.clock_tx.send(|micros| micros.init());
        self.tx.labels_tx.send(|labels| labels.init());
        self.tx
            .resource_targets_tx
            .send_future(|resource_targets| resource_targets.init().boxed());
        self.tx
            .pathfinder_with_planned_roads_tx
            .send_future(|pathfinder| pathfinder.init().boxed());
        self.tx
            .pathfinder_without_planned_roads_tx
            .send_future(|pathfinder| pathfinder.init().boxed());
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
            .build_sim_tx
            .send_future(|build_sim| build_sim.new_game().boxed());
        self.tx
            .prime_mover_tx
            .send_future(|prime_mover| prime_mover.new_game().boxed());
        self.tx
            .setup_new_world_tx
            .send_future(|setup_new_world| setup_new_world.new_game().boxed());
        self.tx
            .simulation_tx
            .send_future(|simulation| simulation.new_game().boxed());
        self.tx
            .visibility_tx
            .send_future(|visibility| visibility.new_game().boxed());
        self.tx
            .world_tx
            .send_future(|world| world.new_game().boxed());
    }

    pub async fn start(&mut self) {
        self.avatars.run_passive(&self.pool).await;
        self.clock.run_passive(&self.pool).await;
        self.nations.run_passive(&self.pool).await;
        self.parameters.run_passive(&self.pool).await;
        self.routes.run_passive(&self.pool).await;
        self.settlements.run_passive(&self.pool).await;
        self.territory.run_passive(&self.pool).await;
        self.world.run_passive(&self.pool).await;

        self.pathfinder_with_planned_roads
            .run_passive(&self.pool)
            .await;
        self.pathfinder_without_planned_roads
            .run_passive(&self.pool)
            .await;

        self.setup_new_world.run_passive(&self.pool).await;
        self.world_artist.run_passive(&self.pool).await;
        self.voyager.run_passive(&self.pool).await;
        self.visibility.run_passive(&self.pool).await;
        self.town_house_artist.run_passive(&self.pool).await;
        self.town_label_artist.run_passive(&self.pool).await;
        self.resource_targets.run_passive(&self.pool).await;
        self.rotate.run_passive(&self.pool).await;
        self.town_builder.run_passive(&self.pool).await;
        self.speed_control.run_passive(&self.pool).await;
        self.build_sim.run_active(&self.pool).await;
        self.simulation.run_active(&self.pool).await;
        self.prime_mover.run_active(&self.pool).await;
        self.pathfinding_avatar_controls
            .run_passive(&self.pool)
            .await;
        self.object_builder.run_passive(&self.pool).await;
        self.labels.run_passive(&self.pool).await;
        self.cheats.run_passive(&self.pool).await;
        self.builder.run_active(&self.pool).await;
        self.basic_road_builder.run_passive(&self.pool).await;
        self.basic_avatar_controls.run_passive(&self.pool).await;
        self.avatar_visibility.run_active(&self.pool).await;
        self.avatar_artist.run_passive(&self.pool).await;
        self.event_forwarder.run_passive(&self.pool).await;

        self.tx.send_clock(|clock| clock.resume()).await;
    }

    pub async fn pause(&mut self) {
        self.tx.send_clock(|clock| clock.pause()).await;

        self.event_forwarder.drain(&self.pool, false).await;
        self.avatar_artist.drain(&self.pool, true).await;
        self.avatar_visibility.drain(&self.pool, true).await;
        self.basic_avatar_controls.drain(&self.pool, true).await;
        self.basic_road_builder.drain(&self.pool, true).await;
        self.builder.drain(&self.pool, true).await;
        self.cheats.drain(&self.pool, true).await;
        self.labels.drain(&self.pool, true).await;
        self.object_builder.drain(&self.pool, true).await;
        self.pathfinding_avatar_controls
            .drain(&self.pool, true)
            .await;
        self.prime_mover.drain(&self.pool, true).await;
        self.simulation.drain(&self.pool, true).await;
        self.build_sim.drain(&self.pool, true).await;
        self.speed_control.drain(&self.pool, true).await;
        self.town_builder.drain(&self.pool, true).await;
        self.rotate.drain(&self.pool, true).await;
        self.resource_targets.drain(&self.pool, true).await;
        self.town_label_artist.drain(&self.pool, true).await;
        self.town_house_artist.drain(&self.pool, true).await;
        self.visibility.drain(&self.pool, true).await;
        self.voyager.drain(&self.pool, true).await;
        self.world_artist.drain(&self.pool, true).await;
        self.setup_new_world.drain(&self.pool, true).await;

        self.pathfinder_without_planned_roads
            .drain(&self.pool, true)
            .await;
        self.pathfinder_with_planned_roads
            .drain(&self.pool, true)
            .await;

        self.world.drain(&self.pool, true).await;
        self.territory.drain(&self.pool, true).await;
        self.settlements.drain(&self.pool, true).await;
        self.parameters.drain(&self.pool, true).await;
        self.routes.drain(&self.pool, true).await;
        self.nations.drain(&self.pool, true).await;
        self.clock.drain(&self.pool, true).await;
        self.avatars.drain(&self.pool, true).await;
    }

    pub async fn save(&mut self, path: &str) {
        self.avatars.object_ref().unwrap().save(path);
        self.build_sim.object_ref().unwrap().save(path);
        self.clock.object_mut().unwrap().save(path);
        self.labels.object_ref().unwrap().save(path);
        self.nations.object_ref().unwrap().save(path);
        self.parameters.object_ref().unwrap().save(path);
        self.prime_mover.object_ref().unwrap().save(path);
        self.routes.object_ref().unwrap().save(path);
        self.settlements.object_ref().unwrap().save(path);
        self.simulation.object_ref().unwrap().save(path);
        self.territory.object_ref().unwrap().save(path);
        self.visibility.object_ref().unwrap().save(path);
        self.world.object_ref().unwrap().save(path);

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
            .route_to_ports
            .read()
            .await
            .save(&format!("{}.route_to_ports", path));
        self.tx
            .traffic
            .read()
            .await
            .save(&format!("{}.traffic", path));
    }

    pub async fn load(&mut self, path: &str) {
        self.avatars.object_mut().unwrap().load(path);
        self.build_sim.object_mut().unwrap().load(path);
        self.clock.object_mut().unwrap().load(path);
        self.labels.object_mut().unwrap().load(path);
        self.nations.object_mut().unwrap().load(path);
        self.prime_mover.object_mut().unwrap().load(path);
        self.routes.object_mut().unwrap().load(path);
        self.settlements.object_mut().unwrap().load(path);
        self.simulation.object_mut().unwrap().load(path);
        self.territory.object_mut().unwrap().load(path);
        self.visibility.object_mut().unwrap().load(path);
        self.world.object_mut().unwrap().load(path);

        *self.tx.build_queue.write().await = <_>::load(&format!("{}.build_queue", path));
        *self.tx.edge_traffic.write().await = <_>::load(&format!("{}.edge_traffic", path));
        *self.tx.route_to_ports.write().await = <_>::load(&format!("{}.route_to_ports", path));
        *self.tx.traffic.write().await = <_>::load(&format!("{}.traffic", path));
    }
}
