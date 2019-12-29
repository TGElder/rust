mod avatar;
mod farms;
mod game;
mod houses;
mod label_editor;
mod pathfinder;
mod road_builder;
mod shore_start;
mod territory;
mod travel_duration;
mod visibility_computer;
mod world;
mod world_gen;

use crate::avatar::*;
use crate::game::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use crate::shore_start::*;
use crate::territory::*;
use crate::world_gen::*;
use isometric::event_handlers::ZoomHandler;
use isometric::IsometricEngine;
use std::env;
use std::thread;

fn new(size: usize, seed: u64, reveal_all: bool) -> (GameState, Vec<GameEvent>) {
    let mut rng = rng(seed);
    let params = GameParams::default();
    let mut world = generate_world(size, &mut rng, &params.world_gen);
    if reveal_all {
        world.reveal_all();
        world.visit_all();
    }
    let avatars = random_avatar_states(&world, &mut rng, 1)
        .into_iter()
        .enumerate()
        .map(|(i, state)| {
            (
                i.to_string(),
                Avatar {
                    name: i.to_string(),
                    state,
                    farm: None,
                },
            )
        })
        .collect();
    let game_state = GameState {
        territory: Territory::new(&world),
        world,
        params,
        game_micros: 0,
        avatars,
        selected_avatar: Some("0".to_string()),
        follow_avatar: true,
        speed: 1.0,
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

    let (game_state, init_events) = if args.len() > 2 {
        let size = args[1].parse().unwrap();
        let seed = args[2].parse().unwrap();
        let reveal_all = args.contains(&"-r".to_string());
        new(size, seed, reveal_all)
    } else if args.len() == 2 {
        load(&args[1])
    } else {
        panic!("Invalid command line arguments");
    };

    let mut engine = IsometricEngine::new(
        "Frontier",
        1024,
        1024,
        game_state.params.world_gen.max_height as f32 + 1.0, // +1 for trees at top
    );

    let avatar_pathfinder = Pathfinder::new(
        &game_state.world,
        AvatarTravelDuration::from_params(&game_state.params.avatar_travel),
    );

    let road_pathfinder = Pathfinder::new(
        &game_state.world,
        AutoRoadTravelDuration::from_params(&game_state.params.auto_road_travel),
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
    game.add_consumer(ObjectArtistHandler::new(game.command_tx()));
    game.add_consumer(VisibilityHandler::new(game.command_tx()));
    game.add_consumer(TerritoryHandler::new(
        avatar_pathfinder_service.command_tx(),
        game.game_state().params.territory_duration,
    ));
    game.add_consumer(FarmCandidateHandler::new(
        avatar_pathfinder_service.command_tx(),
    ));
    game.add_consumer(TownCandidateHandler::new(
        avatar_pathfinder_service.command_tx(),
    ));
    game.add_consumer(FarmAssigner::new(avatar_pathfinder_service.command_tx()));
    game.add_consumer(NaturalRoadBuilder::new(game.command_tx()));

    // Controls
    game.add_consumer(LabelEditorHandler::new(game.command_tx()));
    game.add_consumer(RotateHandler::new(game.command_tx()));
    game.add_consumer(BasicAvatarControls::new(game.command_tx()));
    game.add_consumer(PathfindingAvatarControls::new(
        game.command_tx(),
        avatar_pathfinder_service.command_tx(),
    ));
    game.add_consumer(BasicRoadBuilder::new(game.command_tx()));
    game.add_consumer(PathfindingRoadBuilder::new(
        road_pathfinder_service.command_tx(),
    ));
    game.add_consumer(ObjectBuilder::new(game.command_tx()));
    game.add_consumer(Cheats::new(game.command_tx()));
    game.add_consumer(PrimeMover::new(avatar_pathfinder_service.command_tx()));
    game.add_consumer(Save::new(game.command_tx()));

    game.add_consumer(FollowAvatar::new(game.command_tx()));
    game.add_consumer(avatar_pathfinder_service);
    game.add_consumer(road_pathfinder_service);
    game.add_consumer(SelectAvatar::new(game.command_tx()));
    game.add_consumer(SpeedControl::new(game.command_tx()));

    let game_handle = thread::spawn(move || game.run());
    engine.run();
    game_handle.join().unwrap();
}
