use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use commons::async_trait::async_trait;
use commons::fn_sender::{fn_channel, FnSender};
use commons::futures::executor::ThreadPool;
use commons::futures::future::FutureExt;
use isometric::Command;

use crate::actors::{
    BasicRoadBuilder, ObjectBuilder, TownBuilderActor, TownHouseArtist, TownLabelArtist,
    VisibilityActor, Voyager, WorldArtistActor,
};
use crate::artists::{WorldArtist, WorldArtistParameters};
use crate::avatar::AvatarTravelDuration;
use crate::game::{Game, GameState};
use crate::pathfinder::Pathfinder;
use crate::polysender::Polysender;
use crate::process::{ActiveProcess, PassiveProcess, Persistable, Process};
use crate::road_builder::AutoRoadTravelDuration;
use crate::simulation::builders::{CropsBuilder, RoadBuilder, TownBuilder};
use crate::simulation::demand_fn::{homeland_demand_fn, town_demand_fn};
use crate::simulation::processors::{
    max_abs_population_change, BuildSim, GetDemand, GetRouteChanges, GetRoutes, GetTerritory,
    GetTownTraffic, InstructionLogger, RefreshEdges, RefreshPositions, RemoveTown, StepHomeland,
    StepTown, UpdateCurrentPopulation, UpdateEdgeTraffic, UpdateHomelandPopulation,
    UpdatePositionTraffic, UpdateRouteToPorts, UpdateTown,
};
use crate::simulation::Simulation;
use crate::system::SystemListener;
use crate::traits::{SendGame, SendGameState};

pub struct Configuration {
    pub x: Polysender,
    pub basic_road_builder: PassiveProcess<BasicRoadBuilder<Polysender>>,
    pub object_builder: PassiveProcess<ObjectBuilder<Polysender>>,
    pub simulation: ActiveProcess<Simulation<Polysender>>,
    pub town_builder: PassiveProcess<TownBuilderActor<Polysender>>,
    pub town_house_artist: PassiveProcess<TownHouseArtist<Polysender>>,
    pub town_label_artist: PassiveProcess<TownLabelArtist<Polysender>>,
    pub visibility: PassiveProcess<VisibilityActor<Polysender>>,
    pub voyager: PassiveProcess<Voyager<Polysender>>,
    pub world_artist: PassiveProcess<WorldArtistActor<Polysender>>,
}

impl Configuration {
    pub fn new(
        game_state: &GameState,
        engine_tx: &Sender<Vec<Command>>,
        game_tx: &FnSender<Game>,
        thread_pool: &ThreadPool,
    ) -> Configuration {
        let (basic_road_builder_tx, basic_road_builder_rx) = fn_channel();
        let (object_builder_tx, object_builder_rx) = fn_channel();
        let (simulation_tx, simulation_rx) = fn_channel();
        let (town_builder_tx, town_builder_rx) = fn_channel();
        let (town_house_artist_tx, town_house_artist_rx) = fn_channel();
        let (town_label_artist_tx, town_label_artist_rx) = fn_channel();
        let (visibility_tx, visibility_rx) = fn_channel();
        let (voyager_tx, voyager_rx) = fn_channel();
        let (world_artist_tx, world_artist_rx) = fn_channel();

        let pathfinder_with_planned_roads = Arc::new(RwLock::new(Pathfinder::new(
            &game_state.world,
            AvatarTravelDuration::with_planned_roads_as_roads(&game_state.params.avatar_travel),
        )));
        let pathfinder_without_planned_roads = Arc::new(RwLock::new(Pathfinder::new(
            &game_state.world,
            AvatarTravelDuration::with_planned_roads_ignored(&game_state.params.avatar_travel),
        )));

        let x = Polysender {
            game_tx: game_tx.clone_with_name("polysender"),
            basic_road_builder_tx,
            object_builder_tx,
            simulation_tx,
            town_builder_tx,
            town_house_artist_tx,
            town_label_artist_tx,
            visibility_tx,
            voyager_tx,
            world_artist_tx,
            pathfinder_with_planned_roads: pathfinder_with_planned_roads.clone(),
            pathfinder_without_planned_roads: pathfinder_without_planned_roads.clone(),
        };

        let config = Configuration {
            x: x.clone_with_name("processes"),
            basic_road_builder: PassiveProcess::new(
                BasicRoadBuilder::new(x.clone_with_name("basic_road_builder")),
                basic_road_builder_rx,
            ),
            object_builder: PassiveProcess::new(
                ObjectBuilder::new(x.clone_with_name("object_builder"), game_state.params.seed),
                object_builder_rx,
            ),
            simulation: ActiveProcess::new(
                Simulation::new(
                    x.clone_with_name("simulation"),
                    vec![
                        Box::new(InstructionLogger::new()),
                        Box::new(BuildSim::new(
                            game_tx,
                            vec![
                                Box::new(TownBuilder::new(x.clone_with_name("town_builder"))),
                                Box::new(RoadBuilder::new(x.clone_with_name("road_builder"))),
                                Box::new(CropsBuilder::new(x.clone_with_name("crops_builder"))),
                            ],
                        )),
                        Box::new(StepHomeland::new(game_tx)),
                        Box::new(StepTown::new(game_tx)),
                        Box::new(GetTerritory::new(
                            game_tx,
                            x.clone_with_name("get_territory"),
                        )),
                        Box::new(GetTownTraffic::new(game_tx)),
                        Box::new(UpdateTown::new(x.clone_with_name("update_town"))),
                        Box::new(RemoveTown::new(x.clone_with_name("remove_town"))),
                        Box::new(UpdateHomelandPopulation::new(
                            x.clone_with_name("update_homeland_population"),
                        )),
                        Box::new(UpdateCurrentPopulation::new(
                            x.clone_with_name("update_current_population"),
                            max_abs_population_change,
                        )),
                        Box::new(GetDemand::new(town_demand_fn)),
                        Box::new(GetDemand::new(homeland_demand_fn)),
                        Box::new(GetRoutes::new(
                            game_tx,
                            &pathfinder_with_planned_roads,
                            &pathfinder_without_planned_roads,
                        )),
                        Box::new(GetRouteChanges::new(game_tx)),
                        Box::new(UpdatePositionTraffic::new()),
                        Box::new(UpdateEdgeTraffic::new()),
                        Box::new(RefreshPositions::new(
                            &game_tx,
                            x.clone_with_name("refresh_positions"),
                            thread_pool.clone(),
                        )),
                        Box::new(RefreshEdges::new(
                            &game_tx,
                            x.clone_with_name("refresh_edges"),
                            AutoRoadTravelDuration::from_params(
                                &game_state.params.auto_road_travel,
                            ),
                            &pathfinder_with_planned_roads,
                            thread_pool.clone(),
                        )),
                        Box::new(UpdateRouteToPorts::new(game_tx)),
                    ],
                ),
                simulation_rx,
            ),
            town_builder: PassiveProcess::new(
                TownBuilderActor::new(x.clone_with_name("town_builder_actor")),
                town_builder_rx,
            ),
            town_house_artist: PassiveProcess::new(
                TownHouseArtist::new(
                    x.clone_with_name("town_houses"),
                    engine_tx.clone(),
                    game_state.params.town_artist,
                ),
                town_house_artist_rx,
            ),
            town_label_artist: PassiveProcess::new(
                TownLabelArtist::new(
                    x.clone_with_name("town_labels"),
                    engine_tx.clone(),
                    game_state.params.town_artist,
                ),
                town_label_artist_rx,
            ),
            visibility: PassiveProcess::new(
                VisibilityActor::new(x.clone_with_name("visibility")),
                visibility_rx,
            ),
            voyager: PassiveProcess::new(Voyager::new(x.clone_with_name("voyager")), voyager_rx),
            world_artist: PassiveProcess::new(
                WorldArtistActor::new(
                    x.clone_with_name("world_artist_actor"),
                    engine_tx.clone(),
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
        self.x
            .town_house_artist_tx
            .send_future(|town_house_artist| town_house_artist.init().boxed());
        self.x
            .town_label_artist_tx
            .send_future(|town_label_artist| town_label_artist.init().boxed());
        self.x
            .visibility_tx
            .send_future(|visibility| visibility.init().boxed());
        self.x
            .world_artist_tx
            .send_future(|world_artist| world_artist.init().boxed());
    }

    pub fn new_game(&self) {
        self.x
            .simulation_tx
            .send_future(|simulation| simulation.new_game().boxed());
        self.x
            .visibility_tx
            .send_future(|visibility| visibility.new_game().boxed());
    }

    pub fn load(&mut self, path: &str) {
        self.simulation.load(path);
        self.visibility.load(path);
    }
}

#[async_trait]
impl SystemListener for Configuration {
    async fn start(&mut self, pool: &ThreadPool) {
        self.x
            .send_game_state(|game_state| game_state.speed = game_state.params.default_speed)
            .await;

        self.world_artist.start(pool);
        self.voyager.start(pool);
        self.visibility.start(pool);
        self.town_house_artist.start(pool);
        self.town_label_artist.start(pool);
        self.town_builder.start(pool);
        self.simulation.start(pool);
        self.object_builder.start(pool);
        self.basic_road_builder.start(pool);
    }

    async fn pause(&mut self) {
        self.basic_road_builder.pause().await;
        self.object_builder.pause().await;
        self.simulation.pause().await;
        self.town_builder.pause().await;
        self.town_label_artist.pause().await;
        self.town_house_artist.pause().await;
        self.visibility.pause().await;
        self.voyager.pause().await;
        self.world_artist.pause().await;

        self.x
            .send_game_state(|game_state| game_state.speed = 0.0)
            .await;
    }

    async fn save(&mut self, path: &str) {
        self.simulation.save(path);
        self.visibility.save(path);

        let path = path.to_string();
        self.x.send_game(|game| game.save(path)).await;
    }
}
