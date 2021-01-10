use std::sync::Arc;

use commons::fn_sender::{fn_channel, FnSender};
use futures::executor::ThreadPool;
use futures::future::FutureExt;
use isometric::event_handlers::ZoomHandler;
use isometric::IsometricEngine;

use crate::actors::{
    AvatarArtistActor, BasicAvatarControls, BasicRoadBuilder, Cheats, ObjectBuilder,
    PathfinderService, PathfindingAvatarControls, ResourceTargets, Rotate, TownBuilderActor,
    TownHouseArtist, TownLabelArtist, VisibilityActor, Voyager, WorldArtistActor,
};
use crate::artists::{AvatarArtist, AvatarArtistParams, WorldArtist, WorldArtistParameters};
use crate::avatar::AvatarTravelDuration;
use crate::game::{Game, GameState};
use crate::pathfinder::Pathfinder;
use crate::road_builder::AutoRoadTravelDuration;
use crate::simulation::builders::{CropsBuilder, RoadBuilder, TownBuilder};
use crate::simulation::demand_fn::{homeland_demand_fn, town_demand_fn};
use crate::simulation::processors::{
    max_abs_population_change, BuildCrops, BuildRoad, BuildSim, BuildTown, GetDemand,
    GetRouteChanges, GetRoutes, GetTerritory, GetTownTraffic, InstructionLogger, RemoveCrops,
    RemoveRoad, RemoveTown, StepHomeland, StepTown, UpdateCurrentPopulation, UpdateEdgeTraffic,
    UpdateHomelandPopulation, UpdatePositionTraffic, UpdateRouteToPorts, UpdateTown,
};
use crate::simulation::Simulation;
use crate::system::{EventForwarderActor, EventForwarderConsumer, Polysender};
use crate::traits::{SendGame, SendGameState};
use commons::process::Process;

pub struct System {
    pub tx: Polysender,
    pub pool: ThreadPool,
    pub avatar_artist: Process<AvatarArtistActor<Polysender>>,
    pub basic_avatar_controls: Process<BasicAvatarControls<Polysender>>,
    pub basic_road_builder: Process<BasicRoadBuilder<Polysender>>,
    pub cheats: Process<Cheats<Polysender>>,
    pub event_forwarder: Process<EventForwarderActor>,
    pub object_builder: Process<ObjectBuilder<Polysender>>,
    pub resource_targets: Process<ResourceTargets<Polysender>>,
    pub pathfinding_avatar_controls: Process<PathfindingAvatarControls<Polysender>>,
    pub pathfinder_with_planned_roads: Process<PathfinderService<Polysender, AvatarTravelDuration>>,
    pub pathfinder_without_planned_roads:
        Process<PathfinderService<Polysender, AvatarTravelDuration>>,
    pub rotate: Process<Rotate>,
    pub simulation: Process<Simulation<Polysender>>,
    pub town_builder: Process<TownBuilderActor<Polysender>>,
    pub town_house_artist: Process<TownHouseArtist<Polysender>>,
    pub town_label_artist: Process<TownLabelArtist<Polysender>>,
    pub visibility: Process<VisibilityActor<Polysender>>,
    pub voyager: Process<Voyager<Polysender>>,
    pub world_artist: Process<WorldArtistActor<Polysender>>,
}

impl System {
    pub fn new(
        game_state: &GameState,
        engine: &mut IsometricEngine,
        game_tx: &FnSender<Game>,
        pool: ThreadPool,
    ) -> System {
        let (avatar_artist_tx, avatar_artist_rx) = fn_channel();
        let (basic_avatar_controls_tx, basic_avatar_controls_rx) = fn_channel();
        let (basic_road_builder_tx, basic_road_builder_rx) = fn_channel();
        let (cheats_tx, cheats_rx) = fn_channel();
        let (object_builder_tx, object_builder_rx) = fn_channel();
        let (resource_targets_tx, resource_targets_rx) = fn_channel();
        let (pathfinding_avatar_controls_tx, pathfinding_avatar_controls_rx) = fn_channel();
        let (pathfinder_with_planned_roads_tx, pathfinder_with_planned_roads_rx) = fn_channel();
        let (pathfinder_without_planned_roads_tx, pathfinder_without_planned_roads_rx) =
            fn_channel();
        let (rotate_tx, rotate_rx) = fn_channel();
        let (simulation_tx, simulation_rx) = fn_channel();
        let (town_builder_tx, town_builder_rx) = fn_channel();
        let (town_house_artist_tx, town_house_artist_rx) = fn_channel();
        let (town_label_artist_tx, town_label_artist_rx) = fn_channel();
        let (visibility_tx, visibility_rx) = fn_channel();
        let (voyager_tx, voyager_rx) = fn_channel();
        let (world_artist_tx, world_artist_rx) = fn_channel();

        let tx = Polysender {
            game_tx: game_tx.clone_with_name("polysender"),
            avatar_artist_tx,
            basic_avatar_controls_tx,
            basic_road_builder_tx,
            cheats_tx,
            object_builder_tx,
            resource_targets_tx,
            pathfinding_avatar_controls_tx,
            pathfinder_with_planned_roads_tx,
            pathfinder_without_planned_roads_tx,
            rotate_tx,
            simulation_tx,
            town_builder_tx,
            town_house_artist_tx,
            town_label_artist_tx,
            visibility_tx,
            voyager_tx,
            world_artist_tx,
        };

        let (event_forwarder_tx, event_forwarder_rx) = fn_channel();
        engine.add_event_consumer(EventForwarderConsumer::new(event_forwarder_tx));
        engine.add_event_handler(ZoomHandler::default());

        let config = System {
            tx: tx.clone_with_name("processes"),
            pool,
            avatar_artist: Process::new(
                AvatarArtistActor::new(
                    tx.clone_with_name("avatar_artist"),
                    engine.command_tx(),
                    AvatarArtist::new(AvatarArtistParams::new(&game_state.params.light_direction)),
                ),
                avatar_artist_rx,
            ),
            basic_avatar_controls: Process::new(
                BasicAvatarControls::new(
                    tx.clone_with_name("basic_avatar_controls"),
                    Arc::new(AvatarTravelDuration::with_planned_roads_ignored(
                        &game_state.params.avatar_travel,
                    )),
                ),
                basic_avatar_controls_rx,
            ),
            basic_road_builder: Process::new(
                BasicRoadBuilder::new(tx.clone_with_name("basic_road_builder")),
                basic_road_builder_rx,
            ),
            cheats: Process::new(Cheats::new(tx.clone_with_name("cheats")), cheats_rx),
            event_forwarder: Process::new(
                EventForwarderActor::new(tx.clone_with_name("event_forwarder")),
                event_forwarder_rx,
            ),
            object_builder: Process::new(
                ObjectBuilder::new(tx.clone_with_name("object_builder"), game_state.params.seed),
                object_builder_rx,
            ),
            resource_targets: Process::new(
                ResourceTargets::new(tx.clone_with_name("resource_targets")),
                resource_targets_rx,
            ),
            pathfinding_avatar_controls: Process::new(
                PathfindingAvatarControls::new(
                    tx.clone_with_name("pathfinding_avatar_controls"),
                    Arc::new(AvatarTravelDuration::with_planned_roads_ignored(
                        &game_state.params.avatar_travel,
                    )),
                ),
                pathfinding_avatar_controls_rx,
            ),
            pathfinder_with_planned_roads: Process::new(
                PathfinderService::new(
                    tx.clone_with_name("pathfinder_with_planned_roads"),
                    Pathfinder::new(
                        &game_state.world,
                        AvatarTravelDuration::with_planned_roads_as_roads(
                            &game_state.params.avatar_travel,
                        ),
                    ),
                ),
                pathfinder_with_planned_roads_rx,
            ),
            pathfinder_without_planned_roads: Process::new(
                PathfinderService::new(
                    tx.clone_with_name("pathfinder_without_planned_roads"),
                    Pathfinder::new(
                        &game_state.world,
                        AvatarTravelDuration::with_planned_roads_ignored(
                            &game_state.params.avatar_travel,
                        ),
                    ),
                ),
                pathfinder_without_planned_roads_rx,
            ),
            rotate: Process::new(Rotate::new(engine.command_tx()), rotate_rx),
            simulation: Process::new(
                Simulation::new(
                    tx.clone_with_name("simulation"),
                    vec![
                        Box::new(InstructionLogger::new()),
                        Box::new(BuildSim::new(
                            game_tx,
                            vec![
                                Box::new(TownBuilder::new(tx.clone_with_name("town_builder"))),
                                Box::new(RoadBuilder::new(tx.clone_with_name("road_builder"))),
                                Box::new(CropsBuilder::new(tx.clone_with_name("crops_builder"))),
                            ],
                        )),
                        Box::new(StepHomeland::new(game_tx)),
                        Box::new(StepTown::new(game_tx)),
                        Box::new(GetTerritory::new(
                            game_tx,
                            tx.clone_with_name("get_territory"),
                        )),
                        Box::new(GetTownTraffic::new(game_tx)),
                        Box::new(UpdateTown::new(tx.clone_with_name("update_town"))),
                        Box::new(RemoveTown::new(tx.clone_with_name("remove_town"))),
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
                        Box::new(GetRouteChanges::new(game_tx)),
                        Box::new(UpdatePositionTraffic::new()),
                        Box::new(UpdateEdgeTraffic::new()),
                        Box::new(BuildTown::new(tx.clone_with_name("build_town"))),
                        Box::new(BuildCrops::new(
                            tx.clone_with_name("build_crops"),
                            game_state.params.seed,
                        )),
                        Box::new(RemoveCrops::new(tx.clone_with_name("remove_crops"))),
                        Box::new(BuildRoad::new(
                            tx.clone_with_name("build_road"),
                            Arc::new(AutoRoadTravelDuration::from_params(
                                &game_state.params.auto_road_travel,
                            )),
                        )),
                        Box::new(RemoveRoad::new(tx.clone_with_name("remove_road"))),
                        Box::new(UpdateRouteToPorts::new(game_tx)),
                    ],
                ),
                simulation_rx,
            ),
            town_builder: Process::new(
                TownBuilderActor::new(tx.clone_with_name("town_builder_actor")),
                town_builder_rx,
            ),
            town_house_artist: Process::new(
                TownHouseArtist::new(
                    tx.clone_with_name("town_houses"),
                    engine.command_tx(),
                    game_state.params.town_artist,
                ),
                town_house_artist_rx,
            ),
            town_label_artist: Process::new(
                TownLabelArtist::new(
                    tx.clone_with_name("town_labels"),
                    engine.command_tx(),
                    game_state.params.town_artist,
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
                        &game_state.world,
                        WorldArtistParameters {
                            waterfall_gradient: game_state
                                .params
                                .avatar_travel
                                .max_navigable_river_gradient,
                            ..WorldArtistParameters::default()
                        },
                    ),
                ),
                world_artist_rx,
            ),
        };

        config.send_init_messages();

        config
    }

    pub fn send_init_messages(&self) {
        self.tx
            .avatar_artist_tx
            .send(|avatar_artist| avatar_artist.init());
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
            .simulation_tx
            .send_future(|simulation| simulation.new_game().boxed());
        self.tx
            .visibility_tx
            .send_future(|visibility| visibility.new_game().boxed());
    }

    pub async fn start(&mut self) {
        self.tx
            .send_game_state(|game_state| game_state.speed = game_state.params.default_speed)
            .await;

        self.pathfinder_with_planned_roads
            .run_passive(&self.pool)
            .await;
        self.pathfinder_without_planned_roads
            .run_passive(&self.pool)
            .await;

        self.world_artist.run_passive(&self.pool).await;
        self.voyager.run_passive(&self.pool).await;
        self.visibility.run_passive(&self.pool).await;
        self.town_house_artist.run_passive(&self.pool).await;
        self.town_label_artist.run_passive(&self.pool).await;
        self.resource_targets.run_passive(&self.pool).await;
        self.rotate.run_passive(&self.pool).await;
        self.town_builder.run_passive(&self.pool).await;
        self.simulation.run_active(&self.pool).await;
        self.pathfinding_avatar_controls
            .run_passive(&self.pool)
            .await;
        self.object_builder.run_passive(&self.pool).await;
        self.cheats.run_passive(&self.pool).await;
        self.basic_road_builder.run_passive(&self.pool).await;
        self.basic_avatar_controls.run_passive(&self.pool).await;
        self.avatar_artist.run_passive(&self.pool).await;
        self.event_forwarder.run_passive(&self.pool).await;
    }

    pub async fn pause(&mut self) {
        self.event_forwarder.drain(&self.pool, false).await;
        self.avatar_artist.drain(&self.pool, true).await;
        self.basic_avatar_controls.drain(&self.pool, true).await;
        self.basic_road_builder.drain(&self.pool, true).await;
        self.cheats.drain(&self.pool, true).await;
        self.object_builder.drain(&self.pool, true).await;
        self.pathfinding_avatar_controls
            .drain(&self.pool, true)
            .await;
        self.simulation.drain(&self.pool, true).await;
        self.town_builder.drain(&self.pool, true).await;
        self.rotate.drain(&self.pool, true).await;
        self.resource_targets.drain(&self.pool, true).await;
        self.town_label_artist.drain(&self.pool, true).await;
        self.town_house_artist.drain(&self.pool, true).await;
        self.visibility.drain(&self.pool, true).await;
        self.voyager.drain(&self.pool, true).await;
        self.world_artist.drain(&self.pool, true).await;

        self.pathfinder_without_planned_roads
            .drain(&self.pool, true)
            .await;
        self.pathfinder_with_planned_roads
            .drain(&self.pool, true)
            .await;

        self.tx
            .send_game_state(|game_state| game_state.speed = 0.0)
            .await;
    }

    pub async fn save(&mut self, path: &str) {
        self.simulation.object_ref().unwrap().save(path);
        self.visibility.object_ref().unwrap().save(path);

        let path = path.to_string();
        self.tx.send_game(|game| game.save(path)).await;
    }

    pub fn load(&mut self, path: &str) {
        self.simulation.object_mut().unwrap().load(path);
        self.visibility.object_mut().unwrap().load(path);
    }
}
