use super::*;
use crate::route::*;
use commons::rand::rngs::SmallRng;
use commons::*;
use isometric::{Button, Color, ElementState, VirtualKeyCode};
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
    visible_routes: HashSet<RouteKey>,
    last_outbound: HashMap<RouteKey, bool>,
    frozen_until: HashMap<RouteKey, u128>,
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
        for (name, route) in self.choose_multiple_weighted(candidates, required) {
            self.show_route(game_state, name, route);
        }
    }

    fn get_candidates<'a>(&self, game_state: &'a GameState) -> Vec<(&'a RouteKey, &'a Route)> {
        game_state
            .routes
            .values()
            .flat_map(|route_set| route_set.iter())
            .filter(|(_, route)| route.path.len() > 1)
            .filter(|(key, _)| !is_visible(game_state, &key))
            .filter(|(key, _)| !self.is_frozen(&key))
            .collect()
    }

    fn get_required_count(&self) -> usize {
        self.params.max_visible_routes - self.state.visible_routes.len()
    }

    fn choose_multiple_weighted<'a>(
        &mut self,
        mut candidates: Vec<(&'a RouteKey, &'a Route)>,
        amount: usize,
    ) -> Vec<(&'a RouteKey, &'a Route)> {
        let mut out = vec![];
        while out.len() < amount && !candidates.is_empty() {
            let choice = *candidates
                .choose_weighted(&mut self.rng, |candidate| candidate.1.duration.as_millis())
                .unwrap();
            candidates.retain(|candidate| candidate.0 != choice.0);
            out.push(choice);
        }
        out
    }

    fn show_route(&mut self, game_state: &GameState, key: &RouteKey, route: &Route) {
        let start_at = game_state.game_micros;
        self.state.visible_routes.insert(*key);
        let (color, skin_color) = unwrap_or!(avatar_colors(game_state, key), return);
        let mut positions = route.path.clone();
        let outbound = self.outbound(key);
        if !outbound {
            positions.reverse();
        }
        self.walk_positions(
            *key,
            positions,
            start_at,
            color,
            skin_color,
            if outbound {
                AvatarLoad::None
            } else {
                AvatarLoad::Resource(key.resource)
            },
        );
    }

    fn outbound(&mut self, key: &RouteKey) -> bool {
        let last_outbound = &mut self.state.last_outbound;
        if let Some(outbound) = last_outbound.get_mut(key) {
            *outbound = !*outbound;
            *outbound
        } else {
            let outbound = true;
            last_outbound.insert(*key, outbound);
            outbound
        }
    }

    fn walk_positions(
        &mut self,
        key: RouteKey,
        positions: Vec<V2<usize>>,
        start_at: u128,
        color: Color,
        skin_color: Color,
        load: AvatarLoad,
    ) {
        let pause_at_start = self.params.pause_at_start_of_journey;
        let pause_at_end = self.params.pause_at_end_of_journey;
        let first = *unwrap_or!(positions.first(), return);
        self.game_tx.update(move |game| {
            add_avatar(game, key.to_string(), first, color, skin_color, load);
            walk_positions(
                game,
                key.to_string(),
                positions,
                start_at,
                pause_at_start,
                pause_at_end,
            )
        });
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

    fn is_frozen(&self, key: &RouteKey) -> bool {
        self.state.frozen_until.contains_key(key)
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

fn avatar_colors(game_state: &GameState, key: &RouteKey) -> Option<(Color, Color)> {
    let settlement = game_state.settlements.get(&key.settlement)?;
    let nation = game_state
        .nations
        .get(&settlement.nation)
        .unwrap_or_else(|| panic!("Unknown nation {}", settlement.nation));
    Some((*nation.color(), *nation.skin_color()))
}

fn is_visible(game_state: &GameState, key: &RouteKey) -> bool {
    game_state.avatars.get(&key.to_string()).is_some()
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

fn add_avatar(
    game: &mut Game,
    name: String,
    position: V2<usize>,
    color: Color,
    skin_color: Color,
    load: AvatarLoad,
) {
    let avatar = Avatar {
        name: name.clone(),
        state: AvatarState::Stationary {
            position,
            rotation: Rotation::Up,
        },
        color,
        skin_color,
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
