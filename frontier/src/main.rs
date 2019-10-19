mod avatar;
mod game;
mod houses;
mod label_editor;
mod pathfinder;
mod road_builder;
mod shore_start;
mod travel_duration;
mod visibility_computer;
mod world;
mod world_gen;

use crate::avatar::*;
use crate::game::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use crate::shore_start::*;
use crate::world_gen::*;
use isometric::event_handlers::ZoomHandler;
use isometric::IsometricEngine;
use std::env;
use std::thread;

fn new(size: usize, seed: u64) -> (GameState, Vec<GameEvent>) {
    let mut rng = rng(seed);
    let params = GameParams::default();
    let world = generate_world(size, &mut rng, &params.world_gen);
    let shore_start = shore_start(32, &world, &mut rng);
    let avatar_state = AvatarState::Stationary {
        position: shore_start.at(),
        rotation: shore_start.rotation(),
    };
    let game_state = GameState {
        world,
        params,
        game_micros: 0,
        avatar_state,
        follow_avatar: true,
    };
    let init_events = vec![GameEvent::Init];
    (game_state, init_events)
}

fn load(path: &str) -> (GameState, Vec<GameEvent>) {
    let game_state = GameState::from_file(path);
    let init_events = vec![GameEvent::Load(path.to_string()), GameEvent::Init];
    (game_state, init_events)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let (game_state, init_events) = if args.len() == 3 {
        let size = args[1].parse().unwrap();
        let seed = args[2].parse().unwrap();
        new(size, seed)
    } else if args.len() == 2 {
        load(&args[1])
    } else {
        panic!("Invalid command line arguments");
    };

    let mut engine = IsometricEngine::new(
        "Frontier",
        1024,
        1024,
        game_state.params.world_gen.max_height as f32,
    );

    let avatar_pathfinder = Pathfinder::new(
        &game_state.world,
        AvatarTravelDuration::from_params(&game_state.params.avatar_travel),
    );

    let road_pathfinder = Pathfinder::new(
        &game_state.world,
        AutoRoadTravelDuration::from_params(AutoRoadTravelDurationParams::default()), //TODO should also be from params?
    );

    let mut game = Game::new(game_state, &mut engine);
    for event in init_events {
        game.command_tx().send(GameCommand::Event(event)).unwrap();
    }

    let avatar_pathfinder_service =
        PathfinderServiceEventConsumer::new(game.command_tx(), avatar_pathfinder);

    let road_pathfinder_service =
        PathfinderServiceEventConsumer::new(game.command_tx(), road_pathfinder);

    game.add_consumer(EventHandlerAdapter::new(
        ZoomHandler::default(),
        game.command_tx(),
    ));
    game.add_consumer(WorldArtistHandler::new(game.command_tx()));
    game.add_consumer(AvatarArtistHandler::new(game.command_tx()));
    game.add_consumer(HouseArtistHandler::new(game.command_tx()));
    game.add_consumer(VisibilityHandler::new(game.command_tx()));

    // Controls
    game.add_consumer(LabelEditorHandler::new(game.command_tx()));
    game.add_consumer(RotateHandler::new(game.command_tx()));
    game.add_consumer(BasicAvatarControls::new(game.command_tx()));
    game.add_consumer(PathfindingAvatarControls::new(
        game.command_tx(),
        avatar_pathfinder_service.command_tx(),
    ));
    game.add_consumer(BasicRoadBuilder::new(road_pathfinder_service.command_tx()));
    game.add_consumer(PathfindingRoadBuilder::new(
        road_pathfinder_service.command_tx(),
    ));
    game.add_consumer(HouseBuilderHandler::new(game.command_tx()));
    game.add_consumer(Cheats::new(game.command_tx()));
    game.add_consumer(Save::new(game.command_tx()));

    game.add_consumer(FollowAvatar::new(game.command_tx()));
    game.add_consumer(avatar_pathfinder_service);
    game.add_consumer(road_pathfinder_service);

    let game_handle = thread::spawn(move || game.run());
    engine.run();
    game_handle.join().unwrap();
}
