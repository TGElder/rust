use super::*;
use crate::route::*;
use commons::rand::rngs::SmallRng;
use commons::*;
use isometric::{Button, ElementState, VirtualKeyCode};
use rand::prelude::*;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::time::Duration;

const HANDLE: &str = "prime_mover";

pub struct PrimeMoverParams {
    max_visible_routes: usize,
    pause_at_start_of_journey: Option<Duration>,
    pause_at_end_of_journey: Option<Duration>,
    freeze_duration: Option<Duration>,
}

impl Default for PrimeMoverParams {
    fn default() -> PrimeMoverParams {
        PrimeMoverParams {
            max_visible_routes: 1024,
            pause_at_start_of_journey: Some(Duration::from_secs(60 * 30)),
            pause_at_end_of_journey: Some(Duration::from_secs(60 * 30)),
            freeze_duration: Some(Duration::from_secs(60 * 60)),
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PrimeMoverState {
    visible_routes: HashSet<String>,
    last_outbound: HashMap<String, bool>,
    frozen_until: HashMap<String, u128>,
}

pub struct PrimeMover {
    params: PrimeMoverParams,
    binding: Button,
    game_tx: UpdateSender<Game>,
    state: PrimeMoverState,
    active: bool,
    rng: SmallRng,
}

impl PrimeMover {
    pub fn new(seed: u64, game_tx: &UpdateSender<Game>) -> PrimeMover {
        PrimeMover {
            params: PrimeMoverParams::default(),
            game_tx: game_tx.clone_with_handle(HANDLE),
            state: PrimeMoverState::default(),
            active: true,
            rng: SeedableRng::seed_from_u64(seed),
            binding: Button::Key(VirtualKeyCode::K),
        }
    }

    fn tick(&mut self, game_state: &GameState) {
        if self.active {
            self.update_visible_routes(game_state);
            self.prune_frozen(game_state);
            self.show_routes(game_state);
        }
    }

    fn show_routes(&mut self, game_state: &GameState) {
        let candidates = self.get_candidates(game_state);
        let required = self.get_required_count();
        let chosen = candidates.choose_multiple(&mut self.rng, required);
        for (name, route) in chosen {
            self.show_route(game_state, name, route);
        }
    }

    fn get_required_count(&self) -> usize {
        self.params.max_visible_routes - self.state.visible_routes.len()
    }

    fn get_candidates<'a>(&self, game_state: &'a GameState) -> Vec<(&'a String, &'a Route)> {
        game_state
            .routes
            .iter()
            .filter(|(_, route)| route.path.len() > 1)
            .filter(|(name, _)| !is_visible(game_state, &name))
            .filter(|(name, _)| !self.is_frozen(&name))
            .collect()
    }

    fn show_route(&mut self, game_state: &GameState, name: &str, route: &Route) {
        let start_at = game_state.game_micros;
        self.state.visible_routes.insert(name.to_string());
        if self.outbound(name) {
            self.walk_positions(
                name.to_string(),
                route.path.clone(),
                start_at,
                AvatarLoad::None,
            );
        } else {
            self.walk_positions_reverse(
                name.to_string(),
                route.path.clone(),
                start_at,
                AvatarLoad::Resource(route.resource),
            );
        }
    }

    fn outbound(&mut self, name: &str) -> bool {
        let last_outbound = &mut self.state.last_outbound;
        if let Some(outbound) = last_outbound.get_mut(name) {
            *outbound = !*outbound;
            *outbound
        } else {
            let outbound = true;
            last_outbound.insert(name.to_string(), outbound);
            outbound
        }
    }

    fn walk_positions(
        &mut self,
        name: String,
        positions: Vec<V2<usize>>,
        start_at: u128,
        load: AvatarLoad,
    ) {
        let pause_at_start = self.params.pause_at_start_of_journey;
        let pause_at_end = self.params.pause_at_end_of_journey;
        let first = match positions.first() {
            Some(first) => *first,
            None => return,
        };
        self.game_tx.update(move |game| {
            add_avatar(game, name.clone(), first, load);
            walk_positions(
                game,
                name,
                positions,
                start_at,
                pause_at_start,
                pause_at_end,
            )
        });
    }

    fn walk_positions_reverse(
        &mut self,
        name: String,
        mut positions: Vec<V2<usize>>,
        start_at: u128,
        load: AvatarLoad,
    ) {
        positions.reverse();
        self.walk_positions(name, positions, start_at, load);
    }

    fn update_visible_routes(&mut self, game_state: &GameState) {
        let (visible, mut invisible) = self
            .state
            .visible_routes
            .drain()
            .partition(|name| is_visible(game_state, name));
        self.state.visible_routes = visible;
        if let Some(delay) = self.params.freeze_duration {
            let delay = delay.as_micros();
            invisible.drain().for_each(|name| {
                self.state
                    .frozen_until
                    .insert(name, game_state.game_micros + delay);
            });
        }
    }

    fn is_frozen(&self, name: &str) -> bool {
        self.state.frozen_until.contains_key(name)
    }

    fn prune_frozen(&mut self, game_state: &GameState) {
        self.state.frozen_until = self
            .state
            .frozen_until
            .drain()
            .filter(|(_, freeze_until)| freeze_until > &game_state.game_micros)
            .collect();
    }

    fn get_path(path: &str) -> String {
        format!("{}.prime_mover", path)
    }

    fn save(&mut self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
    }
}

fn is_visible(game_state: &GameState, name: &str) -> bool {
    game_state.avatars.get(name).is_some()
}

fn walk_positions(
    game: &mut Game,
    name: String,
    positions: Vec<V2<usize>>,
    start_at: u128,
    pause_at_start: Option<Duration>,
    pause_at_end: Option<Duration>,
) {
    game.walk_positions(name, positions, start_at, pause_at_start, pause_at_end);
}

fn add_avatar(game: &mut Game, name: String, position: V2<usize>, load: AvatarLoad) {
    let avatar = Avatar {
        name: name.clone(),
        state: AvatarState::Stationary {
            position,
            rotation: Rotation::Up,
        },
        load,
    };
    game.mut_state().avatars.insert(name, avatar);
}

impl GameEventConsumer for PrimeMover {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Tick { .. } => self.tick(game_state),
            GameEvent::Save(path) => self.save(&path),
            GameEvent::Load(path) => self.load(&path),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            ..
        } = *event
        {
            if button == &self.binding {
                self.active = !self.active;
            }
        }
        CaptureEvent::No
    }
}
