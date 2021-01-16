#![type_length_limit = "1870613"]

#[macro_use]
extern crate commons;
#[macro_use]
extern crate futures;

mod actors;
mod artists;
mod avatar;
mod avatars;
mod game;
mod game_event_consumers;
mod homeland_start;
mod label_editor;
mod names;
mod nation;
mod pathfinder;
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

use crate::avatars::Avatars;
use crate::game::*;
use crate::system::{System, SystemController};
use crate::territory::*;
use crate::traits::SendGame;
use crate::world_gen::*;

use commons::async_channel::unbounded;
use commons::fn_sender::fn_channel;
use commons::grid::Grid;
use commons::log::{info, LevelFilter};
use commons::process::run_passive;
use futures::executor::{block_on, ThreadPool};
use futures::FutureExt;
use game_event_consumers::*;
use isometric::{IsometricEngine, IsometricEngineParameters};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

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

    let mut system = System::new(
        &game.game_state(),
        &mut engine,
        game.tx(),
        thread_pool.clone(),
    );
    match parsed_args {
        ParsedArgs::New { .. } => system.new_game(),
        ParsedArgs::Load { path } => system.load(&path),
    }
    let tx = system.tx.clone_with_name("main");

    game.add_consumer(VisibilityFromAvatar::new(
        tx.clone_with_name("visibility_from_avatar"),
    ));

    // Run
    let (system_tx, system_rx) = fn_channel();
    system_tx.send_future(|system: &mut System| system.start().boxed());
    let (shutdown_tx, shutdown_rx) = unbounded();
    engine.add_event_consumer(SystemController::new(system_tx, shutdown_tx));

    let system_handle = run_passive(system, system_rx, shutdown_rx, &thread_pool);
    let game_handle = thread::spawn(move || game.run());
    engine.run();

    // Wait

    info!("Joining system");
    block_on(system_handle);
    info!("Shutting down game");
    block_on(tx.send_game(|game| game.shutdown()));
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
        avatars: Avatars::default(),
        nations: HashMap::new(),
        settlements: HashMap::new(),
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
