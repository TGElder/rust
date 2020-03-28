use super::*;
use commons::rand::rngs::SmallRng;
use commons::*;
use isometric::{Button, ElementState, VirtualKeyCode};
use rand::prelude::*;
use rand::seq::SliceRandom;
use std::collections::HashSet;

const HANDLE: &str = "prime_mover";

pub struct PrimeMover {
    game_tx: UpdateSender<Game>,
    pathfinding: HashSet<String>,
    active: bool,
    rng: SmallRng,
    binding: Button,
}

impl PrimeMover {
    pub fn new(seed: u64, game_tx: &UpdateSender<Game>) -> PrimeMover {
        PrimeMover {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinding: HashSet::new(),
            active: true,
            rng: SeedableRng::seed_from_u64(seed),
            binding: Button::Key(VirtualKeyCode::K),
        }
    }

    fn move_avatars(&mut self, game_state: &GameState) {
        let required = 1024 - self.pathfinding.len();
        let selected = game_state.selected_avatar().map(|avatar| &avatar.name);
        let candidates: Vec<&Avatar> = game_state
            .avatars
            .values()
            .filter(|avatar| Some(&avatar.name) != selected)
            .filter(|avatar| avatar.state == AvatarState::Absent)
            .filter(|avatar| some_non_empty(&avatar.commute))
            .collect();
        let chosen = candidates.choose_multiple(&mut self.rng, required);
        for avatar in chosen {
            self.move_avatar(game_state, avatar);
        }
    }

    fn move_avatar(&mut self, game_state: &GameState, avatar: &Avatar) {
        let start_at = game_state.game_micros;
        if let Some(commute) = &avatar.commute {
            self.pathfinding.insert(avatar.name.clone());
            if self.rng.gen() {
                self.walk_positions(avatar.name.clone(), commute.clone(), start_at);
            } else {
                self.walk_positions_reverse(avatar.name.clone(), commute.clone(), start_at);
            }
        }
    }

    fn walk_positions(&mut self, name: String, positions: Vec<V2<usize>>, start_at: u128) {
        self.game_tx.update(move |game| {
            game.mut_state().avatars.get_mut(&name).unwrap().state = AvatarState::Stationary {
                position: positions[0],
                rotation: Rotation::Up,
            };
            game.walk_positions(name, positions, start_at);
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

    fn update_pathfinding_set(&mut self, game_state: &GameState) {
        self.pathfinding = self
            .pathfinding
            .drain()
            .filter(|name| game_state.avatars[name].state != AvatarState::Absent)
            .collect();
    }
}

fn some_non_empty<T>(vector: &Option<Vec<T>>) -> bool {
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
        if let GameEvent::Tick { .. } = *event {
            if self.active {
                self.update_pathfinding_set(game_state);
                self.move_avatars(game_state);
            }
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

    fn shutdown(&mut self) {}

    fn is_shutdown(&self) -> bool {
        true
    }
}
