#![type_length_limit = "1870613"]

#[macro_use]
extern crate commons;
#[macro_use]
extern crate futures;

mod actors;
mod args;
mod artists;
mod avatar;
mod avatars;
mod build;
mod homeland_start;
mod label_editor;
mod names;
mod nation;
mod parameters;
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

use crate::args::Args;
use crate::parameters::Parameters;
use crate::system::{System, SystemController};

use commons::async_channel::unbounded;
use commons::fn_sender::fn_channel;
use commons::log::{info, LevelFilter};
use commons::process::run_passive;
use futures::executor::{block_on, ThreadPool};
use futures::FutureExt;
use isometric::{IsometricEngine, IsometricEngineParameters};
use simple_logger::SimpleLogger;
use std::env;

fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

    let args = Args::new(env::args().collect());

    let params: Parameters = (&args).into();

    let mut engine = IsometricEngine::new(IsometricEngineParameters {
        title: "Frontier",
        width: 1024,
        height: 1024,
        max_z: params.world_gen.max_height as f32 + 1.2, // +1.2 for resources at top
        label_padding: params.label_padding,
    });

    let thread_pool = ThreadPool::new().unwrap();

    let mut system = System::new(params, &mut engine, thread_pool.clone());
    match args {
        Args::New { .. } => system.new_game(),
        Args::Load { path } => block_on(system.load(&path)),
    }

    // Run
    let (system_tx, system_rx) = fn_channel();
    system_tx.send_future(|system: &mut System| system.start().boxed());
    let (shutdown_tx, shutdown_rx) = unbounded();
    engine.add_event_consumer(SystemController::new(system_tx, shutdown_tx));
    let system_handle = run_passive(system, system_rx, shutdown_rx, &thread_pool);

    engine.run();

    // Wait

    info!("Joining system");
    block_on(system_handle);
}
