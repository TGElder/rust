mod game_params;
mod game_state;
pub mod traits;

use commons::log::warn;
pub use game_params::*;
pub use game_state::*;

use crate::avatar::*;
use crate::settlement::*;
use commons::fn_sender::*;
use commons::futures::executor::block_on;
use commons::V2;
use commons::*;
use isometric::{Command, Event, EventConsumer, IsometricEngine};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub struct TerritoryState {
    pub controller: V2<usize>,
    pub durations: HashMap<V2<usize>, Duration>,
}

pub enum GameEvent {
    NewGame,
    Init,
    Tick { from_micros: u128, to_micros: u128 },
    Save(String),
    Load(String),
    EngineEvent(Arc<Event>),
    SettlementUpdated(Settlement),
}

impl GameEvent {
    fn describe(&self) -> &'static str {
        match self {
            GameEvent::NewGame => "new_game",
            GameEvent::Init => "init",
            GameEvent::Tick { .. } => "tick",
            GameEvent::Save(..) => "save",
            GameEvent::Load(..) => "save",
            GameEvent::EngineEvent(..) => "engine event",
            GameEvent::SettlementUpdated { .. } => "settlement updated",
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
}

pub struct Game {
    game_state: GameState,
    previous_instant: Instant,
    consumers: Vec<Box<dyn GameEventConsumer>>,
    engine_tx: Sender<Vec<Command>>,
    tx: FnSender<Game>,
    rx: FnReceiver<Game>,
    avatar_travel_duration: AvatarTravelDuration,
    run: bool,
}

impl Game {
    pub fn new(
        game_state: GameState,
        engine: &mut IsometricEngine,
        mut init_events: Vec<GameEvent>,
    ) -> Game {
        let (tx, rx) = fn_channel();

        engine.add_event_consumer(EventForwarder::new(tx.clone_with_name("event_forwarder")));

        tx.send(move |game| {
            init_events
                .drain(..)
                .for_each(|event| game.consume_event(event))
        });

        Game {
            previous_instant: Instant::now(),
            avatar_travel_duration: AvatarTravelDuration::with_planned_roads_ignored(
                &game_state.params.avatar_travel,
            ),
            game_state,
            consumers: vec![],
            engine_tx: engine.command_tx(),
            tx,
            rx,
            run: true,
        }
    }

    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    pub fn mut_state(&mut self) -> &mut GameState {
        &mut self.game_state
    }

    pub fn tx(&self) -> &FnSender<Game> {
        &self.tx
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
        let from_micros = self.game_state.game_micros;
        self.update_game_micros();
        let to_micros = self.game_state.game_micros;
        self.update_avatars();
        self.consume_event(GameEvent::Tick {
            from_micros,
            to_micros,
        });
    }

    fn consume_event(&mut self, event: GameEvent) {
        if let GameEvent::EngineEvent(event) = event {
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
        let current_instant = Instant::now();
        let interval = current_instant
            .duration_since(self.previous_instant)
            .as_micros();
        let interval = (interval as f32 * self.game_state.speed).round();
        self.game_state.game_micros += interval as u128;
        self.previous_instant = current_instant;
    }

    fn update_avatars(&mut self) {
        self.evolve_avatars();
        self.prune_avatars();
    }

    fn evolve_avatars(&mut self) {
        let game_micros = &self.game_state.game_micros;
        self.game_state
            .avatars
            .values_mut()
            .for_each(|Avatar { state, .. }| {
                if let Some(new_state) = Self::evolve_avatar(game_micros, state) {
                    *state = new_state;
                }
            });
    }

    fn evolve_avatar(game_micros: &u128, state: &AvatarState) -> Option<AvatarState> {
        if let Some(new_state) = Some(state.evolve(&game_micros)) {
            new_state
        } else {
            None
        }
    }

    fn prune_avatars(&mut self) {
        let selected_avatar_name = self
            .game_state
            .selected_avatar()
            .map(|avatar| avatar.name.to_string());
        self.game_state.avatars.retain(|_, avatar| match avatar {
            Avatar {
                state: AvatarState::Stationary { .. },
                ref name,
                ..
            } if Some(name) != selected_avatar_name.as_ref() => false,
            _ => true,
        });
    }

    pub fn update_settlement(&mut self, settlement: Settlement) {
        self.game_state
            .settlements
            .insert(settlement.position, settlement.clone());
        self.consume_event(GameEvent::SettlementUpdated(settlement));
    }

    pub fn update_avatar_state(&mut self, name: String, new_state: AvatarState) {
        if let Some(avatar) = self.game_state.avatars.get_mut(&name) {
            avatar.state = new_state
        }
    }

    pub fn walk_positions(
        &mut self,
        name: String,
        positions: Vec<V2<usize>>,
        start_at: u128,
        pause_at_start: Option<Duration>,
        pause_at_end: Option<Duration>,
    ) {
        let start_at = start_at.max(self.game_state.game_micros);
        if let Entry::Occupied(mut avatar) = self.game_state.avatars.entry(name) {
            if let Some(new_state) = avatar.get().state.walk_positions(
                &self.game_state.world,
                positions,
                &self.avatar_travel_duration,
                start_at,
                pause_at_start,
                pause_at_end,
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
        while self.run {
            self.on_tick();
            for message in self.rx.get_messages() {
                self.handle_message(message);
            }
        }
    }

    fn handle_message(&mut self, mut message: FnMessage<Game>) {
        let start = Instant::now();
        let name = message.sender_name();
        block_on(message.apply(self));
        log_time(
            name.to_string(),
            start.elapsed(),
            &self.game_state.params.log_duration_threshold,
        );
    }

    pub fn shutdown(&mut self) {
        self.run = false;
    }
}

fn log_time(description: String, duration: Duration, threshold: &Option<Duration>) {
    if let Some(threshold) = threshold {
        if duration >= *threshold {
            warn!("{},{}ms", description, duration.as_millis());
        }
    }
}

struct EventForwarder {
    game_tx: FnSender<Game>,
}

impl EventForwarder {
    pub fn new(game_tx: FnSender<Game>) -> EventForwarder {
        EventForwarder { game_tx }
    }
}

impl EventConsumer for EventForwarder {
    fn consume_event(&mut self, event: Arc<Event>) {
        self.game_tx
            .send(move |game| game.consume_event(GameEvent::EngineEvent(event)));
    }
}
