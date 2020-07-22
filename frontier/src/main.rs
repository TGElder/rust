#![type_length_limit = "1870613"]

#[macro_use]
extern crate commons;

mod artists;
mod avatar;
mod build_service;
mod game;
mod game_event_consumers;
mod homeland_start;
mod label_editor;
mod names;
mod nation;
mod pathfinder;
mod road_builder;
mod route;
mod settlement;
mod simulation_2;
mod territory;
mod travel_duration;
mod update_territory;
mod visibility_computer;
mod world;
mod world_gen;

use crate::avatar::*;
use crate::game::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use crate::territory::*;
use crate::update_territory::TerritoryUpdater;
use crate::world_gen::*;
use build_service::builders::SettlementBuilder;
use build_service::{BuildQueueLoader, BuildService};
use commons::futures::executor::ThreadPool;
use commons::grid::Grid;
use commons::index2d::Vec2D;
use game_event_consumers::*;
use isometric::event_handlers::ZoomHandler;
use isometric::{IsometricEngine, IsometricEngineParameters};
use simulation_2::demand_fn::{homeland_demand_fn, town_demand_fn};
use simulation_2::game_event_consumers::ResourceTargets;
use simulation_2::processors::{
    BuildDestinationTown, GetDemand, GetRouteChanges, GetRoutes, GetTerritory, GetTraffic,
    GetTrafficChanges, InstructionLogger, StepHomeland, StepTown, UpdateCurrentPopulation,
    UpdateTown, VisibilitySim, VisibilitySimConsumer,
};
use simulation_2::{Simulation, SimulationStateLoader};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

fn main() {
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

    let avatar_pathfinder = Arc::new(RwLock::new(Pathfinder::new(
        &game.game_state().world,
        AvatarTravelDuration::from_params(&game.game_state().params.avatar_travel),
    )));
    let road_pathfinder = Arc::new(RwLock::new(Pathfinder::new(
        &game.game_state().world,
        AutoRoadTravelDuration::from_params(&game.game_state().params.auto_road_travel),
    )));

    let territory_updater = TerritoryUpdater::new(
        &game.tx(),
        &avatar_pathfinder,
        game.game_state().params.town_travel_duration,
    );

    let mut builder = BuildService::new(
        game.tx(),
        vec![Box::new(SettlementBuilder::new(
            game.tx(),
            &territory_updater,
        ))],
    );

    let visibility_sim = VisibilitySim::new(game.tx());
    let visibility_sim_consumer = visibility_sim.consumer();

    let mut sim = Simulation::new(vec![
        Box::new(StepHomeland::new(game.tx())),
        Box::new(StepTown::new(game.tx())),
        Box::new(GetTerritory::new(game.tx(), &territory_updater)),
        Box::new(UpdateTown::new(game.tx())),
        Box::new(UpdateCurrentPopulation::new(game.tx())),
        Box::new(GetDemand::new(town_demand_fn)),
        Box::new(GetDemand::new(homeland_demand_fn)),
        Box::new(GetRoutes::new(&avatar_pathfinder)),
        Box::new(GetRouteChanges::new(game.tx())),
        Box::new(GetTrafficChanges::new()),
        Box::new(GetTraffic::new(game.tx())),
        Box::new(BuildDestinationTown::new(game.tx(), builder.tx())),
        Box::new(visibility_sim),
        // Box::new(InstructionLogger::new()),
    ]);

    game.add_consumer(EventHandlerAdapter::new(ZoomHandler::default(), game.tx()));

    // Controls
    game.add_consumer(LabelEditorHandler::new(game.tx()));
    game.add_consumer(RotateHandler::new(game.tx()));
    game.add_consumer(BasicAvatarControls::new(game.tx()));
    game.add_consumer(PathfindingAvatarControls::new(
        game.tx(),
        &avatar_pathfinder,
        thread_pool.clone(),
    ));
    game.add_consumer(BasicRoadBuilder::new(game.tx()));
    game.add_consumer(PathfindingRoadBuilder::new(
        game.tx(),
        &road_pathfinder,
        thread_pool.clone(),
    ));
    game.add_consumer(ObjectBuilder::new(game.game_state().params.seed, game.tx()));
    game.add_consumer(TownBuilder::new(game.tx()));
    game.add_consumer(Cheats::new(game.tx()));
    game.add_consumer(Save::new(game.tx(), builder.tx(), sim.tx()));
    game.add_consumer(SelectAvatar::new(game.tx()));
    game.add_consumer(SpeedControl::new(game.tx()));
    game.add_consumer(ResourceTargets::new(&avatar_pathfinder));

    // Drawing
    game.add_consumer(WorldArtistHandler::new(engine.command_tx()));
    game.add_consumer(AvatarArtistHandler::new(engine.command_tx()));
    game.add_consumer(TownHouses::new(game.tx()));
    game.add_consumer(TownLabels::new(game.tx()));

    // Visibility
    let handler = VisibilityHandler::new(game.tx());
    let from_avatar = VisibilityFromAvatar::new(handler.tx());
    let from_towns = VisibilityFromTowns::new(handler.tx());
    let from_roads = VisibilityFromRoads::new(handler.tx());
    let setup_new_world = SetupNewWorld::new(game.tx(), handler.tx());
    game.add_consumer(from_avatar);
    game.add_consumer(from_towns);
    game.add_consumer(from_roads);
    game.add_consumer(handler);
    game.add_consumer(visibility_sim_consumer);
    game.add_consumer(setup_new_world);

    game.add_consumer(FollowAvatar::new(engine.command_tx(), game.tx()));

    game.add_consumer(PrimeMover::new(game.game_state().params.seed, game.tx()));
    game.add_consumer(Voyager::new(game.tx()));
    game.add_consumer(PathfinderUpdater::new(&avatar_pathfinder));
    game.add_consumer(PathfinderUpdater::new(&road_pathfinder));
    game.add_consumer(BuildQueueLoader::new(builder.tx()));
    game.add_consumer(SimulationStateLoader::new(sim.tx()));

    game.add_consumer(ShutdownHandler::new(
        game.tx(),
        builder.tx(),
        sim.tx(),
        thread_pool,
    ));

    let game_handle = thread::spawn(move || game.run());
    let builder_handle = thread::spawn(move || builder.run());
    let sim_handle = thread::spawn(move || sim.run());

    engine.run();

    println!("Joining sim");
    sim_handle.join().unwrap();
    println!("Joining builder");
    builder_handle.join().unwrap();
    println!("Joining game");
    game_handle.join().unwrap();
}

fn new(power: usize, seed: u64, reveal_all: bool) -> (GameState, Vec<GameEvent>) {
    let mut rng = rng(seed);
    let params = GameParams {
        seed,
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
    let total_visible_positions = if reveal_all {
        world.width() * world.height()
    } else {
        0
    };
    let game_state = GameState {
        territory: Territory::new(&world),
        first_visits: Vec2D::same_size_as(&world, None),
        world,
        game_micros: 0,
        params,
        avatars: HashMap::new(),
        nations: HashMap::new(),
        settlements: HashMap::new(),
        selected_avatar: None,
        follow_avatar: true,
        routes: HashMap::new(),
        speed: 1.0,
        total_visible_positions,
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
