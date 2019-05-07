extern crate nalgebra as na;

mod avatar;
mod game_handler;
mod house_builder;
mod label_editor;
mod roadset;
mod utils;
mod world;
mod world_artist;
mod world_gen;

use crate::game_handler::*;
use crate::world_gen::*;
use isometric::IsometricEngine;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let size = args[1].parse().unwrap();
    let seed = args[2].parse().unwrap();
    let world = generate_world(size, seed);

    let mut engine = IsometricEngine::new("Frontier", 1024, 1024, world.max_height());
    engine.add_event_handler(Box::new(GameHandler::new(world)));

    engine.run();
}
