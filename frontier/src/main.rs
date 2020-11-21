#![type_length_limit = "1870613"]

#[macro_use]
extern crate commons;

mod actors;
mod artists;
mod avatar;
mod event_forwarder;
mod game;
mod game_event_consumers;
mod homeland_start;
mod label_editor;
mod names;
mod nation;
mod pathfinder;
mod polysender;
mod resource;
mod road_builder;
mod route;
mod settlement;
mod simulation;
mod territory;
mod travel_duration;
mod update_territory;
mod visibility_computer;
mod world;
mod world_gen;

use crate::avatar::*;
use crate::event_forwarder::EventForwarder;
use crate::game::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use crate::territory::*;
use crate::update_territory::TerritoryUpdater;
use crate::world_gen::*;
use actors::{BasicRoadBuilder, PauseGame, PauseSim, Save, Visibility, WorldArtistActor};
use artists::{WorldArtist, WorldArtistParameters};
use commons::future::FutureExt;
use commons::futures::executor::{block_on, ThreadPool};
use commons::grid::Grid;
use game_event_consumers::*;
use isometric::event_handlers::ZoomHandler;
use isometric::{IsometricEngine, IsometricEngineParameters};
use polysender::Polysender;
use simple_logger::SimpleLogger;
use simulation::builders::{CropsBuilder, RoadBuilder, SettlementBuilder};
use simulation::demand_fn::{homeland_demand_fn, town_demand_fn};
use simulation::game_event_consumers::ResourceTargets;
use simulation::processors::*;
use simulation::{Simulation, SimulationStateLoader};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

fn main() {
    SimpleLogger::new().init().unwrap();

    let (game_state, init_events) = parse_args(env::args().collect());

    let mut engine = IsometricEngine::new(IsometricEngineParameters {
        title: "Frontier",
        width: 1024,
        height: 1024,
        max_z: game_state.params.world_gen.max_height as f32 + 1.2, // +1.2 for resources at top
        label_padding: game_state.params.label_padding,
    });

    let mut game = Game::new(game_state, &mut engine, init_events);
    let thread_pool = ThreadPool::new().unwrap();

    let pathfinder_with_planned_roads = Arc::new(RwLock::new(Pathfinder::new(
        &game.game_state().world,
        AvatarTravelDuration::with_planned_roads_as_roads(&game.game_state().params.avatar_travel),
    )));
    let pathfinder_without_planned_roads = Arc::new(RwLock::new(Pathfinder::new(
        &game.game_state().world,
        AvatarTravelDuration::with_planned_roads_ignored(&game.game_state().params.avatar_travel),
    )));

    let mut event_forwarder = EventForwarder::new();
    let mut game_event_forwarder = GameEventForwarder::new(thread_pool.clone());

    let mut pause_game = PauseGame::new(event_forwarder.subscribe(), game.tx());

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
    let mut world_artist_actor = WorldArtistActor::new(
        event_forwarder.subscribe(),
        game_event_forwarder.subscribe(),
        game.tx(),
        engine.command_tx(),
        world_artist,
    );

    let mut visibility = Visibility::new(
        event_forwarder.subscribe(),
        game_event_forwarder.subscribe(),
        game.tx(),
    );

    let tx = Polysender {
        game: game.tx().clone(),
        world_artist: world_artist_actor.tx().clone(),
        visibility: visibility.tx().clone(),
        pathfinders: vec![
            pathfinder_with_planned_roads.clone(),
            pathfinder_without_planned_roads.clone(),
        ],
    };

    let mut basic_road_builder = BasicRoadBuilder::new(event_forwarder.subscribe(), &tx);

    let territory_updater = TerritoryUpdater::new(
        &game.tx(),
        &pathfinder_without_planned_roads,
        game.game_state().params.town_travel_duration,
    );

    let builder = BuildSim::new(
        game.tx(),
        vec![
            Box::new(SettlementBuilder::new(game.tx(), &territory_updater)),
            Box::new(RoadBuilder::new(tx.clone_with_name("road_builder"))),
            Box::new(CropsBuilder::new(game.tx())),
        ],
    );

    let visibility_sim = VisibilitySim::new(game.tx());
    let visibility_sim_consumer = visibility_sim.consumer();

    let mut sim = Simulation::new(vec![
        Box::new(InstructionLogger::new()),
        Box::new(builder),
        Box::new(StepHomeland::new(game.tx())),
        Box::new(StepTown::new(game.tx())),
        Box::new(GetTerritory::new(game.tx(), &territory_updater)),
        Box::new(GetTownTraffic::new(game.tx())),
        Box::new(UpdateTown::new(game.tx())),
        Box::new(RemoveTown::new(game.tx())),
        Box::new(UpdateHomelandPopulation::new(game.tx())),
        Box::new(UpdateCurrentPopulation::new(
            game.tx(),
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
        Box::new(RefreshPositions::new(&game.tx())),
        Box::new(RefreshEdges::new(
            &game.tx(),
            tx.clone_with_name("refresh_edges"),
            AutoRoadTravelDuration::from_params(&game.game_state().params.auto_road_travel),
            &pathfinder_with_planned_roads,
            thread_pool.clone(),
        )),
        Box::new(UpdateRouteToPorts::new(game.tx())),
        Box::new(visibility_sim),
    ]);

    let mut pause_sim = PauseSim::new(event_forwarder.subscribe(), sim.tx());
    let mut save = Save::new(event_forwarder.subscribe(), game.tx(), sim.tx());

    game.add_consumer(visibility_sim_consumer);
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
    game.add_consumer(ObjectBuilder::new(game.game_state().params.seed, game.tx()));
    game.add_consumer(TownBuilder::new(game.tx()));
    game.add_consumer(SelectAvatar::new(game.tx()));
    game.add_consumer(SpeedControl::new(game.tx()));
    game.add_consumer(ResourceTargets::new(&pathfinder_with_planned_roads));

    // Drawing

    game.add_consumer(AvatarArtistHandler::new(engine.command_tx()));
    game.add_consumer(TownHouses::new(game.tx()));
    game.add_consumer(TownLabels::new(game.tx()));

    game.add_consumer(FollowAvatar::new(engine.command_tx(), game.tx()));

    game.add_consumer(PrimeMover::new(game.game_state().params.seed, game.tx()));
    game.add_consumer(Voyager::new(game.tx()));
    game.add_consumer(PathfinderUpdater::new(&pathfinder_with_planned_roads));
    game.add_consumer(PathfinderUpdater::new(&pathfinder_without_planned_roads));
    game.add_consumer(SimulationStateLoader::new(sim.tx()));

    game.add_consumer(ShutdownHandler::new(
        game.tx(),
        sim.tx(),
        thread_pool.clone(),
    ));

    // Visibility
    let from_avatar = VisibilityFromAvatar::new(visibility.tx());
    let from_towns = VisibilityFromTowns::new(visibility.tx());
    let setup_new_world = SetupNewWorld::new(game.tx(), visibility.tx());
    game.add_consumer(from_avatar);
    game.add_consumer(from_towns);
    game.add_consumer(setup_new_world);

    game.add_consumer(Cheats::new(game.tx(), visibility.tx()));

    game.add_consumer(game_event_forwarder);
    game.add_consumer(WorldArtistHandler::new(
        world_artist_actor.tx(),
        thread_pool.clone(),
    ));

    engine.add_event_consumer(event_forwarder);

    // Run

    let game_handle = thread::spawn(move || game.run());

    let (basic_road_builder_run, basic_road_builder_handle) =
        async move { basic_road_builder.run().await }.remote_handle();
    thread_pool.spawn_ok(basic_road_builder_run);

    let (pause_game_run, pause_game_handle) = async move { pause_game.run().await }.remote_handle();
    thread_pool.spawn_ok(pause_game_run);

    let (pause_sim_run, pause_sim_handle) = async move { pause_sim.run().await }.remote_handle();
    thread_pool.spawn_ok(pause_sim_run);

    let (save_run, save_handle) = async move { save.run().await }.remote_handle();
    thread_pool.spawn_ok(save_run);

    let (visibility_run, visibility_handle) = async move { visibility.run().await }.remote_handle();
    thread_pool.spawn_ok(visibility_run);

    let (world_artist_actor_run, world_artist_actor_handle) =
        async move { world_artist_actor.run().await }.remote_handle();
    thread_pool.spawn_ok(world_artist_actor_run);

    let (sim_run, sim_handle) = async move { sim.run().await }.remote_handle();
    thread_pool.spawn_ok(sim_run);

    engine.run();

    // Wait

    println!("Joining actors");
    block_on(async {
        join!(
            basic_road_builder_handle,
            pause_game_handle,
            pause_sim_handle,
            save_handle,
            sim_handle,
            world_artist_actor_handle,
            visibility_handle
        )
    });
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
    let mut init_events = vec![GameEvent::NewGame, GameEvent::Init];
    let mut world = generate_world(power, &mut rng, &params.world_gen);
    if reveal_all {
        world.reveal_all();
        init_events.push(GameEvent::CellsRevealed {
            selection: CellSelection::All,
            by: "init",
        });
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

#[allow(clippy::comparison_chain)]
fn parse_args(args: Vec<String>) -> (GameState, Vec<GameEvent>) {
    if args.len() > 2 {
        let power = args[1].parse().unwrap();
        let seed = args[2].parse().unwrap();
        let reveal_all = args.contains(&"-r".to_string());
        new(power, seed, reveal_all)
    } else if args.len() == 2 {
        load(&args[1])
    } else {
        panic!("Invalid command line arguments");
    }
}
