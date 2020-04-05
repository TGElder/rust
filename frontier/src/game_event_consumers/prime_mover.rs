use super::*;
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
    max_visible_avatars: usize,
    pause_at_start_of_journey: Option<Duration>,
    pause_at_end_of_journey: Option<Duration>,
    freeze_duration: Option<Duration>,
}

impl Default for PrimeMoverParams {
    fn default() -> PrimeMoverParams {
        PrimeMoverParams {
            max_visible_avatars: 1024,
            pause_at_start_of_journey: Some(Duration::from_secs(60 * 30)),
            pause_at_end_of_journey: Some(Duration::from_secs(60 * 30)),
            freeze_duration: Some(Duration::from_secs(60 * 60)),
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PrimeMoverState {
    visible_avatars: HashSet<String>,
    last_directions: HashMap<String, bool>,
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
            self.update_visible_avatars(game_state);
            self.prune_frozen(game_state);
            self.move_avatars(game_state);
        }
    }

    fn move_avatars(&mut self, game_state: &GameState) {
        let candidates = self.get_candidates(game_state);
        let required = self.get_required_count();
        let chosen = candidates.choose_multiple(&mut self.rng, required);
        for avatar in chosen {
            self.move_avatar(game_state, avatar);
        }
    }

    fn get_required_count(&self) -> usize {
        self.params.max_visible_avatars - self.state.visible_avatars.len()
    }

    fn get_candidates<'a>(&self, game_state: &'a GameState) -> Vec<&'a Avatar> {
        let selected = game_state.selected_avatar().map(|avatar| &avatar.name);
        game_state
            .avatars
            .values()
            .filter(|avatar| avatar.state == AvatarState::Absent)
            .filter(|avatar| Some(&avatar.name) != selected)
            .filter(|avatar| !self.is_frozen(&avatar.name))
            .filter(|avatar| some_and_non_empty(&avatar.route))
            .collect()
    }

    fn move_avatar(&mut self, game_state: &GameState, avatar: &Avatar) {
        let start_at = game_state.game_micros;
        if let Some(route) = &avatar.route {
            self.state.visible_avatars.insert(avatar.name.clone());
            if self.next_direction(avatar.name.clone()) {
                self.walk_positions(avatar.name.clone(), route.clone(), start_at);
            } else {
                self.walk_positions_reverse(avatar.name.clone(), route.clone(), start_at);
            }
        }
    }

    fn next_direction(&mut self, avatar_name: String) -> bool {
        let last_directions = &mut self.state.last_directions;
        let rng = &mut self.rng;
        let direction = last_directions
            .entry(avatar_name)
            .or_insert_with(|| rng.gen());
        *direction = !*direction;
        *direction
    }

    fn walk_positions(&mut self, name: String, positions: Vec<V2<usize>>, start_at: u128) {
        let pause_at_start = self.params.pause_at_start_of_journey;
        let pause_at_end = self.params.pause_at_end_of_journey;
        self.game_tx.update(move |game| {
            game.mut_state().avatars.get_mut(&name).unwrap().state = AvatarState::Stationary {
                position: positions[0],
                rotation: Rotation::Up,
            };
            game.walk_positions(name, positions, start_at, pause_at_start, pause_at_end);
        });
    }

    fn walk_positions_reverse(
        &mut self,
        name: String,
        mut positions: Vec<V2<usize>>,
        start_at: u128,
    ) {
        positions.reverse();
        self.walk_positions(name, positions, start_at);
    }

    fn update_visible_avatars(&mut self, game_state: &GameState) {
        let (visible, mut invisible) = self
            .state
            .visible_avatars
            .drain()
            .partition(|name| is_visible(game_state, name));
        self.state.visible_avatars = visible;
        if let Some(delay) = self.params.freeze_duration {
            let delay = delay.as_micros();
            invisible.drain().for_each(|name| {
                self.state
                    .frozen_until
                    .insert(name, game_state.game_micros + delay);
            });
        }
    }

    fn is_frozen(&self, avatar_name: &str) -> bool {
        self.state.frozen_until.contains_key(avatar_name)
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

fn is_visible(game_state: &GameState, avatar_name: &str) -> bool {
    match game_state.avatars.get(avatar_name) {
        Some(avatar) => avatar.state != AvatarState::Absent,
        None => false,
    }
}

fn some_and_non_empty<T>(vector: &Option<Vec<T>>) -> bool {
    if let Some(vector) = vector {
        !vector.is_empty()
    } else {
        false
    }
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
