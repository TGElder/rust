mod game_params;
mod game_state;
mod pathfinder_service;

pub use game_params::*;
pub use game_state::*;
pub use pathfinder_service::*;

use crate::avatar::*;
use crate::road_builder::*;
use crate::territory::*;
use crate::world::*;
use commons::grid::Grid;
use commons::update::*;
use commons::V2;
use commons::*;
use isometric::{Command, Event, EventConsumer, IsometricEngine};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

const UPDATE_CHANNEL_BOUND: usize = 10_000;

pub enum CellSelection {
    All,
    Some(Vec<V2<usize>>),
}

pub struct TerritoryState {
    pub controller: V2<usize>,
    pub durations: HashMap<V2<usize>, Duration>,
}

pub enum GameEvent {
    Init,
    Tick,
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
}

impl GameEvent {
    fn describe(&self) -> &'static str {
        match self {
            GameEvent::Init => "init",
            GameEvent::Tick { .. } => "tick",
            GameEvent::Save(..) => "save",
            GameEvent::Load(..) => "save",
            GameEvent::EngineEvent(..) => "engine event",
            GameEvent::CellsVisited(..) => "cells visited",
            GameEvent::CellsRevealed(..) => "cells revealed",
            GameEvent::RoadsUpdated(..) => "roads updated",
            GameEvent::ObjectUpdated { .. } => "object updated",
            GameEvent::TerritoryChanged(..) => "territory changed",
        }
    }
}

pub enum CaptureEvent {
    Yes,
    No,
}

pub trait GameEventConsumer: Send {
    fn name(&self) -> &'static str;
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent;
    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent;
    fn shutdown(&mut self);
    fn is_shutdown(&self) -> bool;
}

pub struct Game {
    game_state: GameState,
    real_time: Instant,
    consumers: Vec<Box<dyn GameEventConsumer>>,
    engine_tx: Sender<Vec<Command>>,
    update_tx: UpdateSender<Game>,
    update_rx: UpdateReceiver<Game>,
    avatar_travel_duration: AvatarTravelDuration,
    run: bool,
}

impl Game {
    pub fn new(
        game_state: GameState,
        engine: &mut IsometricEngine,
        mut init_events: Vec<GameEvent>,
    ) -> Game {
        let (update_tx, update_rx) = update_channel(UPDATE_CHANNEL_BOUND);

        engine.add_event_consumer(EventForwarder::new(
            update_tx.clone_with_handle("event_forwarder"),
        ));

        update_tx.update(move |game| {
            init_events
                .drain(..)
                .for_each(|event| game.consume_event(event))
        });

        Game {
            real_time: Instant::now(),
            avatar_travel_duration: AvatarTravelDuration::from_params(
                &game_state.params.avatar_travel,
            ),
            game_state,
            consumers: vec![],
            engine_tx: engine.command_tx(),
            update_tx,
            update_rx,
            run: true,
        }
    }

    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    pub fn mut_state(&mut self) -> &mut GameState {
        &mut self.game_state
    }

    pub fn update_tx(&self) -> &UpdateSender<Game> {
        &self.update_tx
    }

    pub fn add_consumer<T>(&mut self, consumer: T)
    where
        T: GameEventConsumer + 'static,
    {
        self.consumers.push(Box::new(consumer));
    }

    pub fn send_engine_commands(&mut self, commands: Vec<Command>) {
        self.engine_tx.send(commands).unwrap();
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
                let capture = consumer.consume_engine_event(&self.game_state, event.clone());
                if let CaptureEvent::Yes = capture {
                    return;
                }
            }
        } else {
            let log_duration_threshold = &self.game_state.params.log_duration_threshold;
            for consumer in self.consumers.iter_mut() {
                let start = Instant::now();
                let capture = consumer.consume_game_event(&self.game_state, &event);
                log_time(
                    format!("event,{},{}", event.describe(), consumer.name()),
                    start.elapsed(),
                    log_duration_threshold,
                );
                if let CaptureEvent::Yes = capture {
                    return;
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
        let selected_avatar_name = self
            .game_state
            .selected_avatar()
            .map(|avatar| avatar.name.to_string());
        self.game_state.avatars.values_mut().for_each(
            |Avatar {
                 state, ref name, ..
             }| {
                if let Some(new_state) = Self::evolve_avatar(game_micros, state) {
                    if let AvatarState::Stationary { .. } = new_state {
                        if Some(name) != selected_avatar_name.as_ref() {
                            *state = AvatarState::Absent;
                            return;
                        }
                    }
                    *state = new_state;
                }
            },
        )
    }

    fn evolve_avatar(game_micros: &u128, state: &AvatarState) -> Option<AvatarState> {
        if let Some(new_state) = Some(state.evolve(&game_micros)) {
            new_state
        } else {
            None
        }
    }

    pub fn visit_all_cells(&mut self) {
        self.game_state.world.visit_all();
        self.consume_event(GameEvent::CellsVisited(CellSelection::All));
    }

    pub fn visit_cells(&mut self, cells: Vec<V2<usize>>) {
        let mut send = vec![];
        for position in cells {
            let world_cell = match self.game_state.world.mut_cell(&position) {
                Some(world_cell) => world_cell,
                None => continue,
            };
            if !world_cell.visited {
                world_cell.visited = true;
                send.push(position);
            }
        }
        if send.is_empty() {
            return;
        }
        self.consume_event(GameEvent::CellsVisited(CellSelection::Some(send)));
    }

    pub fn reveal_all_cells(&mut self) {
        self.game_state.world.reveal_all();
        self.consume_event(GameEvent::CellsRevealed(CellSelection::All));
    }

    pub fn reveal_cells(&mut self, cells: Vec<V2<usize>>) {
        let mut send = vec![];
        for position in cells {
            if let Some(world_cell) = self.game_state.world.mut_cell(&position) {
                if !world_cell.visible {
                    world_cell.visible = true;
                    send.push(position);
                }
            }
        }
        if send.is_empty() {
            return;
        }
        self.consume_event(GameEvent::CellsRevealed(CellSelection::Some(send)));
    }

    pub fn update_roads(&mut self, result: RoadBuilderResult) {
        result.update_roads(&mut self.game_state.world);
        self.consume_event(GameEvent::RoadsUpdated(result));
    }

    fn build_object(&mut self, object: WorldObject, position: V2<usize>) -> bool {
        if let Some(cell) = self.game_state.world.mut_cell(&position) {
            if let WorldObject::None = cell.object {
                cell.object = object;
                return true;
            }
        }
        false
    }

    fn destroy_object(&mut self, object: WorldObject, position: V2<usize>) -> bool {
        if let Some(cell) = self.game_state.world.mut_cell(&position) {
            if object == cell.object {
                cell.object = WorldObject::None;
                return true;
            }
        }
        false
    }

    pub fn update_object(&mut self, object: WorldObject, position: V2<usize>, build: bool) -> bool {
        let success = if build {
            self.build_object(object, position)
        } else {
            self.destroy_object(object, position)
        };
        if let WorldObject::House(..) = object {
            if build {
                self.game_state.territory.add_controller(position);
            } else {
                self.set_territory(vec![TerritoryState {
                    controller: position,
                    durations: HashMap::new(),
                }]);
                self.game_state.territory.remove_controller(&position);
            }
        };
        if success {
            self.consume_event(GameEvent::ObjectUpdated {
                object,
                position,
                built: build,
            })
        }
        success
    }

    pub fn clear_object(&mut self, position: V2<usize>) -> bool {
        let cell = match self.game_state.world.get_cell(&position) {
            Some(cell) => *cell,
            _ => return false,
        };
        if cell.object != WorldObject::None {
            self.update_object(cell.object, position, false)
        } else {
            true
        }
    }

    pub fn set_territory(&mut self, states: Vec<TerritoryState>) {
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
        self.consume_event(GameEvent::TerritoryChanged(changes));
    }

    pub fn update_avatar_state(&mut self, name: String, new_state: AvatarState) {
        if let Some(avatar) = self.game_state.avatars.get_mut(&name) {
            avatar.state = new_state
        }
    }

    pub fn walk_positions(&mut self, name: String, positions: Vec<V2<usize>>, start_at: u128) {
        let start_at = start_at.max(self.game_state.game_micros);
        if let Entry::Occupied(mut avatar) = self.game_state.avatars.entry(name) {
            if let Some(new_state) = avatar.get().state.walk_positions(
                &self.game_state.world,
                positions,
                &self.avatar_travel_duration,
                start_at,
            ) {
                avatar.get_mut().state = new_state;
            }
        }
    }

    pub fn save(&mut self, path: String) {
        self.game_state.to_file(&path);
        self.consume_event(GameEvent::Save(path));
    }

    pub fn run(&mut self) {
        loop {
            if self.run {
                self.consume_event(GameEvent::Tick);
            } else {
                self.progress_shutdown();
                if self.consumers.is_empty() {
                    return;
                }
            }
            for update in self.update_rx.get_updates() {
                self.handle_update(update);
            }
        }
    }

    fn handle_update(&mut self, update: Arm<Update<Game>>) {
        let start = Instant::now();
        let handle = update.lock().unwrap().sender_handle();
        process_update(update, self);
        log_time(
            handle.to_string(),
            start.elapsed(),
            &self.game_state.params.log_duration_threshold,
        );
    }

    pub fn shutdown(&mut self) {
        self.run = false;
        self.shutdown_next_consumer();
    }

    fn shutdown_next_consumer(&mut self) -> bool {
        if let Some(consumer) = self.consumers.first_mut() {
            consumer.shutdown();
            return true;
        }
        false
    }

    fn progress_shutdown(&mut self) {
        if let Some(consumer) = self.consumers.first() {
            if consumer.is_shutdown() {
                println!("{} is done", consumer.name());
                self.consumers.remove(0);
                self.shutdown_next_consumer();
            }
        }
    }
}

fn log_time(description: String, duration: Duration, threshold: &Duration) {
    if duration >= *threshold {
        println!("{},{}ms", description, duration.as_millis());
    }
}

struct EventForwarder {
    game_tx: UpdateSender<Game>,
}

impl EventForwarder {
    pub fn new(game_tx: UpdateSender<Game>) -> EventForwarder {
        EventForwarder { game_tx }
    }
}

impl EventConsumer for EventForwarder {
    fn consume_event(&mut self, event: Arc<Event>) {
        self.game_tx
            .update(move |game| game.consume_event(GameEvent::EngineEvent(event)));
    }
}
