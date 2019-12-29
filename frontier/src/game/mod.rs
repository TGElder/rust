mod game_params;
mod game_state;
mod handlers;
mod pathfinder_service;

pub use game_params::*;
pub use game_state::*;
pub use handlers::*;
pub use pathfinder_service::*;

use crate::avatar::*;
use crate::road_builder::*;
use crate::territory::*;
use crate::world::*;
use commons::edge::*;
use commons::grid::Grid;
use commons::{M, V2};
use isometric::{Command, Event, EventConsumer, IsometricEngine};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub enum CellSelection {
    All,
    Some(Vec<V2<usize>>),
}

pub struct TerritoryState {
    controller: V2<usize>,
    durations: HashMap<V2<usize>, Duration>,
}

pub enum GameEvent {
    Init,
    Save(String),
    Load(String),
    EngineEvent(Arc<Event>),
    CellsVisited(CellSelection),
    CellsRevealed(CellSelection),
    RoadsUpdated(RoadBuilderResult),
    ObjectUpdated {
        object: WorldObject,
        position: V2<usize>,
        built: bool,
    },
    TerritoryChanged(Vec<TerritoryChange>),
    Traffic {
        name: String,
        edges: Vec<Edge>,
    },
}

pub enum GameCommand {
    Event(GameEvent),
    EngineCommands(Vec<Command>),
    VisitCells(CellSelection),
    RevealCells(CellSelection),
    UpdateRoads(RoadBuilderResult),
    UpdateObject {
        object: WorldObject,
        position: V2<usize>,
        build: bool,
    },
    SetTerritory(Vec<TerritoryState>),
    AddAvatar {
        name: String,
        avatar: Avatar,
    },
    UpdateAvatar {
        name: String,
        new_state: AvatarState,
    },
    SetAvatarFarm {
        name: String,
        farm: Option<V2<usize>>,
    },
    WalkPositions {
        name: String,
        positions: Vec<V2<usize>>,
        start_at: u128,
    },
    SelectAvatar(String),
    FollowAvatar(bool),
    Update(Box<dyn FnOnce(&mut GameState) -> Vec<GameCommand> + Send>),
    Shutdown,
}

pub enum CaptureEvent {
    Yes,
    No,
}

pub trait GameEventConsumer: Send {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent;
    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent;
}

pub struct Game {
    game_state: GameState,
    real_time: Instant,
    consumers: Vec<Box<dyn GameEventConsumer>>,
    engine_tx: Sender<Vec<Command>>,
    command_tx: Sender<GameCommand>,
    command_rx: Receiver<GameCommand>,
    avatar_travel_duration: AvatarTravelDuration,
}

impl Game {
    pub fn new(game_state: GameState, engine: &mut IsometricEngine) -> Game {
        let (command_tx, command_rx) = mpsc::channel();

        let event_forwarder = EventForwarder::new(command_tx.clone());
        engine.add_event_consumer(event_forwarder);

        let mut out = Game {
            real_time: Instant::now(),
            avatar_travel_duration: AvatarTravelDuration::from_params(
                &game_state.params.avatar_travel,
            ),
            game_state,
            consumers: vec![],
            engine_tx: engine.command_tx(),
            command_tx,
            command_rx,
        };

        out.add_consumer(ShutdownHandler::new(out.command_tx()));

        out
    }

    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    pub fn command_tx(&self) -> Sender<GameCommand> {
        self.command_tx.clone()
    }

    pub fn add_consumer<T>(&mut self, consumer: T)
    where
        T: GameEventConsumer + 'static,
    {
        self.consumers.push(Box::new(consumer));
    }

    fn on_tick(&mut self) {
        let from = self.game_state.game_micros;
        self.update_game_micros();
        let to = self.game_state.game_micros;
        self.process_visited_cells(&from, &to);
        self.evolve_avatars();
    }

    fn consume_event(&mut self, event: GameEvent) {
        if let GameEvent::EngineEvent(event) = event {
            if let Event::Tick = *event {
                self.on_tick();
            }
            for consumer in self.consumers.iter_mut() {
                match consumer.consume_engine_event(&self.game_state, event.clone()) {
                    CaptureEvent::Yes => return,
                    CaptureEvent::No => (),
                }
            }
        } else {
            for consumer in self.consumers.iter_mut() {
                match consumer.consume_game_event(&self.game_state, &event) {
                    CaptureEvent::Yes => return,
                    CaptureEvent::No => (),
                }
            }
        }
    }

    fn update_game_micros(&mut self) {
        let current_time = Instant::now();
        let interval = current_time.duration_since(self.real_time).as_micros();
        let interval = (interval as f32 * self.game_state.speed).round();
        self.game_state.game_micros += interval as u128;
        self.real_time = current_time;
    }

    fn process_visited_cells(&mut self, from: &u128, to: &u128) {
        let mut visited_cells = vec![];
        for avatar in self.game_state.avatars.values() {
            match &avatar.state {
                AvatarState::Walking(path) => {
                    let edges = path.edges_between_times(from, to);
                    if !edges.is_empty() {
                        edges.iter().for_each(|edge| visited_cells.push(*edge.to()));
                        self.command_tx
                            .send(GameCommand::Event(GameEvent::Traffic {
                                name: avatar.name.clone(),
                                edges,
                            }))
                            .unwrap();
                    }
                }
                AvatarState::Stationary { position, .. } => visited_cells.push(*position),
                _ => (),
            }
        }
        self.visit_cells(visited_cells);
    }

    fn evolve_avatars(&mut self) {
        let game_micros = &self.game_state.game_micros;
        self.game_state
            .avatars
            .values_mut()
            .for_each(|Avatar { state, .. }| {
                if let Some(new_state) = Self::evolve_avatar(game_micros, state) {
                    *state = new_state
                }
            })
    }

    fn evolve_avatar(game_micros: &u128, state: &AvatarState) -> Option<AvatarState> {
        if let Some(new_state) = Some(state.evolve(&game_micros)) {
            new_state
        } else {
            None
        }
    }

    fn visit_all_cells(&mut self) {
        self.game_state.world.visit_all();
        self.command_tx
            .send(GameCommand::Event(GameEvent::CellsVisited(
                CellSelection::All,
            )))
            .unwrap();
    }

    fn visit_cells(&mut self, cells: Vec<V2<usize>>) {
        let mut send = vec![];
        for position in cells {
            if let Some(world_cell) = self.game_state.world.mut_cell(&position) {
                if !world_cell.visited {
                    world_cell.visited = true;
                    send.push(position);
                }
            }
        }
        self.command_tx
            .send(GameCommand::Event(GameEvent::CellsVisited(
                CellSelection::Some(send),
            )))
            .unwrap();
    }

    fn reveal_all_cells(&mut self) {
        self.game_state.world.reveal_all();
        self.command_tx
            .send(GameCommand::Event(GameEvent::CellsRevealed(
                CellSelection::All,
            )))
            .unwrap();
    }

    fn reveal_cells(&mut self, cells: Vec<V2<usize>>) {
        let mut send = vec![];
        for position in cells {
            if let Some(world_cell) = self.game_state.world.mut_cell(&position) {
                if !world_cell.visible {
                    world_cell.visible = true;
                    send.push(position);
                }
            }
        }
        self.command_tx
            .send(GameCommand::Event(GameEvent::CellsRevealed(
                CellSelection::Some(send),
            )))
            .unwrap();
    }

    fn build_object(&mut self, object: WorldObject, position: V2<usize>) -> Option<GameEvent> {
        if let Some(cell) = self.game_state.world.mut_cell(&position) {
            if let WorldObject::None = cell.object {
                cell.object = object;
                return Some(GameEvent::ObjectUpdated {
                    object,
                    position,
                    built: true,
                });
            }
        }
        None
    }

    fn destroy_object(&mut self, object: WorldObject, position: V2<usize>) -> Option<GameEvent> {
        if let Some(cell) = self.game_state.world.mut_cell(&position) {
            if object == cell.object {
                cell.object = WorldObject::None;
                return Some(GameEvent::ObjectUpdated {
                    object,
                    position,
                    built: false,
                });
            }
        }
        None
    }

    fn update_object(&mut self, object: WorldObject, position: V2<usize>, build: bool) {
        let event = if build {
            self.build_object(object, position)
        } else {
            self.destroy_object(object, position)
        };
        event
            .into_iter()
            .for_each(|event| self.command_tx.send(GameCommand::Event(event)).unwrap());
    }

    fn set_territory(&mut self, states: Vec<TerritoryState>) {
        let mut changes = vec![];
        for TerritoryState {
            controller,
            durations,
        } in states
        {
            changes.append(&mut self.game_state.territory.set_durations(
                controller,
                &durations,
                &self.game_state.game_micros,
            ));
        }
        self.command_tx
            .send(GameCommand::Event(GameEvent::TerritoryChanged(changes)))
            .unwrap();
    }

    fn add_avatar(&mut self, name: String, avatar: Avatar) {
        self.game_state.avatars.insert(name, avatar);
    }

    fn update_avatar(&mut self, name: String, new_state: AvatarState) {
        if let Some(avatar) = self.game_state.avatars.get_mut(&name) {
            avatar.state = new_state
        }
    }

    fn set_avatar_farm(&mut self, name: String, farm: Option<V2<usize>>) {
        if let Some(farm) = farm {
            if let Some(cell) = self.game_state.world.mut_cell(&farm) {
                if let (Some(avatar), WorldObject::None) =
                    (self.game_state.avatars.get_mut(&name), WorldObject::None)
                {
                    cell.object = WorldObject::Farm;
                    avatar.farm = Some(farm);
                    let event = GameEvent::ObjectUpdated {
                        object: WorldObject::Farm,
                        position: farm,
                        built: true,
                    };
                    self.command_tx.send(GameCommand::Event(event)).unwrap()
                }
            }
        }
    }

    fn walk_positions(&mut self, name: String, positions: Vec<V2<usize>>, start_at: u128) {
        let start_at = start_at.max(self.game_state.game_micros);
        if let Entry::Occupied(mut avatar) = self.game_state.avatars.entry(name.clone()) {
            if let Some(new_state) = avatar.get().state.walk_positions(
                &self.game_state.world,
                positions.clone(),
                &self.avatar_travel_duration,
                start_at,
            ) {
                avatar.get_mut().state = new_state;
            }
        }
    }

    fn select_avatar(&mut self, name: String) {
        self.game_state.selected_avatar = Some(name);
    }

    fn set_follow_avatar(&mut self, follow_avatar: bool) {
        self.game_state.follow_avatar = follow_avatar;
    }

    fn update(&mut self, function: Box<dyn FnOnce(&mut GameState) -> Vec<GameCommand> + Send>) {
        let commands = function(&mut self.game_state);
        for command in commands {
            self.command_tx.send(command).unwrap();
        }
    }

    pub fn run(&mut self) {
        loop {
            let command = self.command_rx.recv().unwrap();
            match command {
                GameCommand::Event(event) => self.consume_event(event),
                GameCommand::EngineCommands(commands) => self.engine_tx.send(commands).unwrap(),
                GameCommand::VisitCells(selection) => {
                    match selection {
                        CellSelection::All => self.visit_all_cells(),
                        CellSelection::Some(cells) => self.visit_cells(cells),
                    };
                }
                GameCommand::RevealCells(selection) => {
                    match selection {
                        CellSelection::All => self.reveal_all_cells(),
                        CellSelection::Some(cells) => self.reveal_cells(cells),
                    };
                }
                GameCommand::UpdateRoads(result) => {
                    result.update_roads(&mut self.game_state.world);
                    self.command_tx
                        .send(GameCommand::Event(GameEvent::RoadsUpdated(result)))
                        .unwrap();
                }
                GameCommand::UpdateObject {
                    object,
                    position,
                    build,
                } => self.update_object(object, position, build),
                GameCommand::SetTerritory(states) => self.set_territory(states),
                GameCommand::AddAvatar { name, avatar } => self.add_avatar(name, avatar),
                GameCommand::UpdateAvatar { name, new_state } => {
                    self.update_avatar(name, new_state)
                }
                GameCommand::SetAvatarFarm { name, farm } => self.set_avatar_farm(name, farm),
                GameCommand::WalkPositions {
                    name,
                    positions,
                    start_at,
                } => self.walk_positions(name, positions, start_at),
                GameCommand::SelectAvatar(name) => self.select_avatar(name),
                GameCommand::FollowAvatar(follow_avatar) => self.set_follow_avatar(follow_avatar),
                GameCommand::Update(function) => self.update(function),
                GameCommand::Shutdown => return,
            }
        }
    }
}

struct ShutdownHandler {
    command_tx: Sender<GameCommand>,
}

impl ShutdownHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> ShutdownHandler {
        ShutdownHandler { command_tx }
    }
}

impl GameEventConsumer for ShutdownHandler {
    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Shutdown = *event {
            self.command_tx.send(GameCommand::Shutdown).unwrap();
        }
        CaptureEvent::No
    }
}

struct EventForwarder {
    command_tx: Sender<GameCommand>,
}

impl EventForwarder {
    pub fn new(command_tx: Sender<GameCommand>) -> EventForwarder {
        EventForwarder { command_tx }
    }
}

impl EventConsumer for EventForwarder {
    fn consume_event(&mut self, event: Arc<Event>) {
        self.command_tx
            .send(GameCommand::Event(GameEvent::EngineEvent(event)))
            .unwrap();
    }
}
