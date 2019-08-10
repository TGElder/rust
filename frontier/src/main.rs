mod avatar;
mod game_handler;
mod house_builder;
mod label_editor;
mod pathfinder;
mod road_builder;
mod shore_start;
mod travel_duration;
mod visibility_computer;
mod world;
mod world_gen;

use crate::game_handler::*;
use crate::world_gen::*;
use commons::*;
use isometric::drawing::*;
use isometric::{AsyncEventHandler, IsometricEngine};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut game_handler = if args.len() == 2 {
        GameHandler::load(Load::from_file(&args[1]))
    } else {
        let size = args[1].parse().unwrap();
        let seed = args[2].parse().unwrap();
        let mut rng = rng(seed);
        let world_gen_params = WorldGenParameters::default();
        let world = generate_world(size, &mut rng, &world_gen_params);
        GameHandler::new(world, world_gen_params, &mut rng)
    };

    let world = game_handler.world();
    let overlay =
        NodeTerrainColoring::from_data(M::from_fn(world.width(), world.height(), |x, y| {
            world.get_cell(&v2(x, y)).unwrap().climate.groundwater()
        }));
    game_handler.set_overlay(overlay);

    let mut engine =
        IsometricEngine::new("Frontier", 1024, 1024, game_handler.world().max_height());
    engine.add_event_handler(Box::new(AsyncEventHandler::new(Box::new(game_handler))));

    engine.run();
}
