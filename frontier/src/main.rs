#![type_length_limit = "1870613"]

#[macro_use]
extern crate commons;

mod actors;
mod artists;
mod avatar;
mod configuration;
mod event_forwarder;
mod event_forwarder_2;
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

use crate::configuration::Configuration;
use crate::event_forwarder::EventForwarder;
use crate::event_forwarder_2::EventForwarder2;
use crate::game::*;
use crate::system::System;
use crate::territory::*;
use crate::traits::SendGame;
use crate::world_gen::*;

use commons::future::FutureExt;
use commons::futures::executor::{block_on, ThreadPool};
use commons::grid::Grid;
use commons::log::info;
use game_event_consumers::*;
use isometric::event_handlers::ZoomHandler;
use isometric::{IsometricEngine, IsometricEngineParameters};
use simple_logger::SimpleLogger;
use simulation::game_event_consumers::ResourceTargets;
use std::collections::HashMap;
use std::env;
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

    let mut config = Configuration::new(
        &game.game_state(),
        &engine.command_tx(),
        game.tx(),
        &thread_pool,
    );
    match parsed_args {
        ParsedArgs::New { .. } => config.new_game(),
        ParsedArgs::Load { path } => config.load(&path),
    }
    let x = config.x.clone_with_name("main");

    game.add_consumer(EventHandlerAdapter::new(ZoomHandler::default(), game.tx()));

    // Controls
    game.add_consumer(LabelEditorHandler::new(game.tx()));
    game.add_consumer(RotateHandler::new(game.tx()));
    game.add_consumer(BasicAvatarControls::new(game.tx()));
    game.add_consumer(PathfindingAvatarControls::new(
        game.tx(),
        &config.x.pathfinder_without_planned_roads,
        thread_pool.clone(),
    ));
    game.add_consumer(SelectAvatar::new(game.tx()));
    game.add_consumer(SpeedControl::new(game.tx()));
    game.add_consumer(ResourceTargets::new(
        &config.x.pathfinder_with_planned_roads,
    ));

    // Drawing

    game.add_consumer(AvatarArtistHandler::new(engine.command_tx()));

    game.add_consumer(FollowAvatar::new(engine.command_tx(), game.tx()));

    game.add_consumer(PrimeMover::new(game.game_state().params.seed, game.tx()));
    game.add_consumer(PathfinderUpdater::new(&x.pathfinder_with_planned_roads));
    game.add_consumer(PathfinderUpdater::new(&x.pathfinder_without_planned_roads));

    // Visibility
    let from_avatar = VisibilityFromAvatar::new(x.clone_with_name("visibility_from_avatar"));
    let setup_new_world = SetupNewWorld::new(x.clone_with_name("setup_new_world"));
    game.add_consumer(from_avatar);
    game.add_consumer(setup_new_world);

    game.add_consumer(Cheats::new(
        x.clone_with_name("cheats"),
        thread_pool.clone(),
    ));

    let mut event_forwarder = EventForwarder::new();
    let mut system = System::new(event_forwarder.subscribe(), thread_pool.clone(), config);

    engine.add_event_consumer(EventForwarder2::new(x.clone_with_name("event_forwarder")));
    engine.add_event_consumer(event_forwarder);

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
