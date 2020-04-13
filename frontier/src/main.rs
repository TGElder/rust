#![type_length_limit = "1870613"]

mod avatar;
mod game;
mod game_event_consumers;
mod houses;
mod label_editor;
mod pathfinder;
mod road_builder;
mod shore_start;
mod simulation;
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
use commons::futures::executor::ThreadPool;
use commons::update::*;
use game_event_consumers::*;
use isometric::event_handlers::ZoomHandler;
use isometric::IsometricEngine;
use simulation::*;
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let (game_state, init_events) = parse_args(env::args().collect());

    let mut engine = IsometricEngine::new(
        "Frontier",
        1024,
        1024,
        game_state.params.world_gen.max_height as f32 + 1.0, // +1 for trees at top
    );

    let mut game = Game::new(game_state, &mut engine, init_events);
    let thread_pool = ThreadPool::new().unwrap();

    let avatar_pathfinder = Arc::new(Mutex::new(Pathfinder::new(
        &game.game_state().world,
        AvatarTravelDuration::from_params(&game.game_state().params.avatar_travel),
    )));
    let mut avatar_pathfinder_service = PathfinderService::new(avatar_pathfinder.clone());
    let road_pathfinder = Arc::new(Mutex::new(Pathfinder::new(
        &game.game_state().world,
        AutoRoadTravelDuration::from_params(&game.game_state().params.auto_road_travel),
    )));
    let mut road_pathfinder_service = PathfinderService::new(road_pathfinder.clone());

    let mut sim = create_simulation(
        &game.game_state().params,
        game.update_tx(),
        avatar_pathfinder_service.update_tx(),
    );

    game.add_consumer(EventHandlerAdapter::new(
        ZoomHandler::default(),
        game.update_tx(),
    ));

    // Controls
    game.add_consumer(LabelEditorHandler::new(game.update_tx()));
    game.add_consumer(RotateHandler::new(game.update_tx()));
    game.add_consumer(BasicAvatarControls::new(game.update_tx()));
    game.add_consumer(PathfindingAvatarControls::new(
        game.update_tx(),
        avatar_pathfinder_service.update_tx(),
        thread_pool.clone(),
    ));
    game.add_consumer(BasicRoadBuilder::new(game.update_tx()));
    game.add_consumer(PathfindingRoadBuilder::new(
        game.update_tx(),
        road_pathfinder_service.update_tx(),
        thread_pool.clone(),
    ));
    game.add_consumer(ObjectBuilder::new(
        game.game_state().params.seed,
        game.game_state().params.house_color,
        game.update_tx(),
    ));
    game.add_consumer(Cheats::new(game.update_tx()));
    game.add_consumer(Save::new(game.update_tx(), sim.update_tx()));
    game.add_consumer(FollowAvatar::new(engine.command_tx(), game.update_tx()));
    game.add_consumer(SelectAvatar::new(game.update_tx()));
    game.add_consumer(SpeedControl::new(game.update_tx()));

    // Drawing
    game.add_consumer(WorldArtistHandler::new(engine.command_tx()));
    game.add_consumer(AvatarArtistHandler::new(engine.command_tx()));
    game.add_consumer(ObjectArtistHandler::new(engine.command_tx()));
    game.add_consumer(VisibilityHandler::new(game.update_tx()));

    game.add_consumer(PrimeMover::new(
        game.game_state().params.seed,
        game.update_tx(),
    ));
    game.add_consumer(PathfinderUpdater::new(avatar_pathfinder));
    game.add_consumer(PathfinderUpdater::new(road_pathfinder));
    game.add_consumer(FarmCandidateHandler::new(
        avatar_pathfinder_service.update_tx(),
    ));
    game.add_consumer(SimulationManager::new(sim.update_tx()));
    game.add_consumer(ShutdownHandler::new(
        avatar_pathfinder_service.update_tx(),
        road_pathfinder_service.update_tx(),
        game.update_tx(),
        sim.update_tx(),
        thread_pool,
    ));

    let avatar_pathfinder_handle = thread::spawn(move || avatar_pathfinder_service.run());
    let road_pathfinder_handle = thread::spawn(move || road_pathfinder_service.run());
    let game_handle = thread::spawn(move || game.run());
    let sim_handle = thread::spawn(move || sim.run());

    engine.run();

    sim_handle.join().unwrap();
    game_handle.join().unwrap();
    road_pathfinder_handle.join().unwrap();
    avatar_pathfinder_handle.join().unwrap();
}

fn new(size: usize, seed: u64, reveal_all: bool) -> (GameState, Vec<GameEvent>) {
    let mut rng = rng(seed);
    let params = GameParams::new(seed);
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
                    birthday: params.sim.start_year,
                    state,
                    farm: None,
                    children: vec![],
                    route: None,
                },
            )
        })
        .collect();
    let game_state = GameState {
        territory: Territory::new(&world),
        world,
        game_micros: 0,
        params,
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

#[allow(clippy::comparison_chain)]
fn parse_args(args: Vec<String>) -> (GameState, Vec<GameEvent>) {
    if args.len() > 2 {
        let size = args[1].parse().unwrap();
        let seed = args[2].parse().unwrap();
        let reveal_all = args.contains(&"-r".to_string());
        new(size, seed, reveal_all)
    } else if args.len() == 2 {
        load(&args[1])
    } else {
        panic!("Invalid command line arguments");
    }
}

fn create_simulation(
    params: &GameParams,
    game_tx: &UpdateSender<Game>,
    pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
) -> Simulation {
    let seed = params.seed;
    let house_color = params.house_color;

    let territory_sim = TerritorySim::new(
        game_tx,
        pathfinder_tx,
        params
            .town_exclusive_duration
            .max(params.town_travel_duration),
    );
    let farm_unassigner_sim = FarmUnassignerSim::new(game_tx);
    let farm_assigner_sim = FarmAssignerSim::new(game_tx, pathfinder_tx, seed);
    let children_sim = ChildrenSim::new(params.sim.children, seed, game_tx, pathfinder_tx);
    let route_sim = RouteSim::new(params.sim.route, seed, game_tx, pathfinder_tx);
    let natural_town_sim = NaturalTownSim::new(
        params.sim.natural_town,
        house_color,
        game_tx,
        territory_sim.clone(),
    );
    let natural_road_sim = NaturalRoadSim::new(
        params.sim.natural_road,
        AutoRoadTravelDuration::from_params(&params.auto_road_travel),
        game_tx,
    );

    Simulation::new(
        params.sim.start_year,
        vec![
            Box::new(territory_sim),
            Box::new(farm_unassigner_sim),
            Box::new(farm_assigner_sim),
            Box::new(children_sim),
            Box::new(route_sim),
            Box::new(natural_town_sim),
            Box::new(natural_road_sim),
        ],
    )
}
