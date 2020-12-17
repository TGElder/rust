#![type_length_limit = "1870613"]

#[macro_use]
extern crate commons;

mod actors;
mod artists;
mod avatar;
mod event_forwarder;
mod event_forwarder_2;
mod frontier;
mod game;
mod game_event_consumers;
mod homeland_start;
mod label_editor;
mod names;
mod nation;
mod pathfinder;
mod polysender;
mod process;
mod resource;
mod road_builder;
mod route;
mod settlement;
mod simulation;
mod system;
mod territory;
mod traits;
mod travel_duration;
mod visibility_computer;
mod world;
mod world_gen;

use crate::actors::{
    BasicRoadBuilder, ObjectBuilder, TownBuilderActor, TownHouseArtist, TownLabelArtist,
    VisibilityActor, Voyager, WorldArtistActor,
};
use crate::avatar::*;
use crate::event_forwarder::EventForwarder;
use crate::event_forwarder_2::EventForwarder2;
use crate::frontier::Frontier;
use crate::game::*;
use crate::pathfinder::*;
use crate::process::{ActiveProcess, PassiveProcess};
use crate::road_builder::*;
use crate::system::System;
use crate::territory::*;
use crate::traits::SendGame;
use crate::world_gen::*;
use artists::{WorldArtist, WorldArtistParameters};
use commons::fn_sender::fn_channel;
use commons::future::FutureExt;
use commons::futures::executor::{block_on, ThreadPool};
use commons::grid::Grid;
use commons::log::info;
use game_event_consumers::*;
use isometric::event_handlers::ZoomHandler;
use isometric::{IsometricEngine, IsometricEngineParameters};
use polysender::Polysender;
use simple_logger::SimpleLogger;
use simulation::builders::{CropsBuilder, RoadBuilder, TownBuilder};
use simulation::demand_fn::{homeland_demand_fn, town_demand_fn};
use simulation::game_event_consumers::ResourceTargets;
use simulation::processors::*;
use simulation::Simulation;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

fn main() {
    SimpleLogger::new().init().unwrap();

    let parsed_args = parse_args(env::args().collect());

    let (game_state, init_events) = match &parsed_args {
        ParsedArgs::New {
            power,
            seed,
            reveal_all,
        } => new(*power, *seed, *reveal_all),
        ParsedArgs::Load { path } => load(&path),
    };

    let mut engine = IsometricEngine::new(IsometricEngineParameters {
        title: "Frontier",
        width: 1024,
        height: 1024,
        max_z: game_state.params.world_gen.max_height as f32 + 1.2, // +1.2 for resources at top
        label_padding: game_state.params.label_padding,
    });

    let mut game = Game::new(game_state, &mut engine, init_events);
    let thread_pool = ThreadPool::new().unwrap();

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
        &game.game_state().world,
        AvatarTravelDuration::with_planned_roads_as_roads(&game.game_state().params.avatar_travel),
    )));
    let pathfinder_without_planned_roads = Arc::new(RwLock::new(Pathfinder::new(
        &game.game_state().world,
        AvatarTravelDuration::with_planned_roads_ignored(&game.game_state().params.avatar_travel),
    )));

    let x = Polysender {
        game_tx: game.tx().clone_with_name("polysender"),
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

    let mut event_forwarder = EventForwarder::new();

    let world_artist = WorldArtist::new(
        &game.game_state().world,
        WorldArtistParameters {
            waterfall_gradient: game
                .game_state()
                .params
                .avatar_travel
                .max_navigable_river_gradient,
            ..WorldArtistParameters::default()
        },
    );

    let town_house_artist = PassiveProcess::new(
        TownHouseArtist::new(
            x.clone_with_name("town_houses"),
            engine.command_tx(),
            game.game_state().params.town_artist,
        ),
        town_house_artist_rx,
    );

    let basic_road_builder = PassiveProcess::new(
        BasicRoadBuilder::new(x.clone_with_name("basic_road_builder")),
        basic_road_builder_rx,
    );

    let town_builder = PassiveProcess::new(
        TownBuilderActor::new(x.clone_with_name("town_builder_actor")),
        town_builder_rx,
    );

    let object_builder = PassiveProcess::new(
        ObjectBuilder::new(
            x.clone_with_name("object_builder"),
            game.game_state().params.seed,
        ),
        object_builder_rx,
    );

    let town_label_artist = PassiveProcess::new(
        TownLabelArtist::new(
            x.clone_with_name("town_labels"),
            engine.command_tx(),
            game.game_state().params.town_artist,
        ),
        town_label_artist_rx,
    );

    let visibility = PassiveProcess::new(
        VisibilityActor::new(x.clone_with_name("visibility")),
        visibility_rx,
    );

    let voyager = PassiveProcess::new(Voyager::new(x.clone_with_name("voyager")), voyager_rx);

    let world_artist = PassiveProcess::new(
        WorldArtistActor::new(
            x.clone_with_name("world_artist_actor"),
            engine.command_tx(),
            world_artist,
        ),
        world_artist_rx,
    );

    let builder = BuildSim::new(
        game.tx(),
        vec![
            Box::new(TownBuilder::new(x.clone_with_name("town_builder"))),
            Box::new(RoadBuilder::new(x.clone_with_name("road_builder"))),
            Box::new(CropsBuilder::new(x.clone_with_name("crops_builder"))),
        ],
    );

    let simulation = ActiveProcess::new(
        Simulation::new(
            x.clone_with_name("simulation"),
            vec![
                Box::new(InstructionLogger::new()),
                Box::new(builder),
                Box::new(StepHomeland::new(game.tx())),
                Box::new(StepTown::new(game.tx())),
                Box::new(GetTerritory::new(
                    game.tx(),
                    x.clone_with_name("get_territory"),
                )),
                Box::new(GetTownTraffic::new(game.tx())),
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
                    game.tx(),
                    &pathfinder_with_planned_roads,
                    &pathfinder_without_planned_roads,
                )),
                Box::new(GetRouteChanges::new(game.tx())),
                Box::new(UpdatePositionTraffic::new()),
                Box::new(UpdateEdgeTraffic::new()),
                Box::new(RefreshPositions::new(
                    &game.tx(),
                    x.clone_with_name("refresh_positions"),
                    thread_pool.clone(),
                )),
                Box::new(RefreshEdges::new(
                    &game.tx(),
                    x.clone_with_name("refresh_edges"),
                    AutoRoadTravelDuration::from_params(&game.game_state().params.auto_road_travel),
                    &pathfinder_with_planned_roads,
                    thread_pool.clone(),
                )),
                Box::new(UpdateRouteToPorts::new(game.tx())),
            ],
        ),
        simulation_rx,
    );

    let mut frontier = Frontier {
        x: x.clone_with_name("processes"),
        basic_road_builder,
        object_builder,
        simulation,
        town_builder,
        town_house_artist,
        town_label_artist,
        visibility,
        voyager,
        world_artist,
    };

    frontier.send_init_messages();

    match parsed_args {
        ParsedArgs::New { .. } => frontier.new_game(),
        ParsedArgs::Load { path } => frontier.load(&path),
    }

    let mut system = System::new(event_forwarder.subscribe(), thread_pool.clone(), frontier);

    game.add_consumer(EventHandlerAdapter::new(ZoomHandler::default(), game.tx()));

    // Controls
    game.add_consumer(LabelEditorHandler::new(game.tx()));
    game.add_consumer(RotateHandler::new(game.tx()));
    game.add_consumer(BasicAvatarControls::new(game.tx()));
    game.add_consumer(PathfindingAvatarControls::new(
        game.tx(),
        &pathfinder_without_planned_roads,
        thread_pool.clone(),
    ));
    game.add_consumer(SelectAvatar::new(game.tx()));
    game.add_consumer(SpeedControl::new(game.tx()));
    game.add_consumer(ResourceTargets::new(&pathfinder_with_planned_roads));

    // Drawing

    game.add_consumer(AvatarArtistHandler::new(engine.command_tx()));

    game.add_consumer(FollowAvatar::new(engine.command_tx(), game.tx()));

    game.add_consumer(PrimeMover::new(game.game_state().params.seed, game.tx()));
    game.add_consumer(PathfinderUpdater::new(&pathfinder_with_planned_roads));
    game.add_consumer(PathfinderUpdater::new(&pathfinder_without_planned_roads));

    // Visibility
    let from_avatar = VisibilityFromAvatar::new(x.clone_with_name("visibility_from_avatar"));
    let setup_new_world = SetupNewWorld::new(x.clone_with_name("setup_new_world"));
    game.add_consumer(from_avatar);
    game.add_consumer(setup_new_world);

    game.add_consumer(Cheats::new(
        x.clone_with_name("cheats"),
        thread_pool.clone(),
    ));

    engine.add_event_consumer(event_forwarder);
    engine.add_event_consumer(EventForwarder2::new(x.clone_with_name("event_forwarder")));

    // Run

    let game_handle = thread::spawn(move || game.run());

    let (system_run, system_handle) = async move { system.run().await }.remote_handle();
    thread_pool.spawn_ok(system_run);

    engine.run();

    // Wait

    info!("Joining system");
    block_on(system_handle);

    info!("Shutting down game");
    block_on(x.send_game(|game| game.shutdown()));
    info!("Shut down game");
    println!("Joining game");
    game_handle.join().unwrap();
}

fn new(power: usize, seed: u64, reveal_all: bool) -> (GameState, Vec<GameEvent>) {
    let mut rng = rng(seed);
    let params = GameParams {
        seed,
        reveal_all,
        homeland_distance: Duration::from_secs((3600.0 * 2f32.powf(power as f32)) as u64),
        ..GameParams::default()
    };
    let init_events = vec![GameEvent::NewGame, GameEvent::Init];
    let mut world = generate_world(power, &mut rng, &params.world_gen);
    if reveal_all {
        world.reveal_all();
    }
    let visible_land_positions = if reveal_all {
        world.width() * world.height()
    } else {
        0
    };
    let game_state = GameState {
        territory: Territory::new(&world),
        world,
        game_micros: 0,
        speed: params.default_speed,
        params,
        avatars: HashMap::new(),
        nations: HashMap::new(),
        settlements: HashMap::new(),
        selected_avatar: None,
        follow_avatar: true,
        routes: HashMap::new(),
        visible_land_positions,
    };

    (game_state, init_events)
}

fn load(path: &str) -> (GameState, Vec<GameEvent>) {
    let game_state = GameState::from_file(path);
    let init_events = vec![GameEvent::Load(path.to_string()), GameEvent::Init];
    (game_state, init_events)
}

enum ParsedArgs {
    New {
        power: usize,
        seed: u64,
        reveal_all: bool,
    },
    Load {
        path: String,
    },
}

#[allow(clippy::comparison_chain)]
fn parse_args(args: Vec<String>) -> ParsedArgs {
    if args.len() > 2 {
        ParsedArgs::New {
            power: args[1].parse().unwrap(),
            seed: args[2].parse().unwrap(),
            reveal_all: args.contains(&"-r".to_string()),
        }
    } else if args.len() == 2 {
        ParsedArgs::Load {
            path: args[1].clone(),
        }
    } else {
        panic!("Invalid command line arguments");
    }
}
