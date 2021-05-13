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
mod bridge;
mod build;
mod homeland_start;
mod label_editor;
mod names;
mod nation;
mod parameters;
mod pathfinder;
mod resource;
mod resource_gen;
mod road_builder;
mod route;
mod services;
mod settlement;
mod simulation;
mod system;
mod territory;
mod traffic;
mod traits;
mod travel_duration;
mod visibility_computer;
mod visited;
mod world;
mod world_gen;

use crate::args::Args;
use crate::parameters::Parameters;
use crate::system::System;

use commons::log::LevelFilter;
use futures::executor::block_on;
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

    let mut system = System::new(params, &mut engine);
    match args {
        Args::New { .. } => system.new_game(),
        Args::Load { path, .. } => block_on(system.load(&path)),
    }
    let system_handle = system.run();

    engine.run();

    block_on(system_handle);
}
