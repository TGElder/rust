use super::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, MouseButton, VirtualKeyCode};
use std::default::Default;

const HANDLE: &str = "pathfinder_avatar_controls";

pub struct PathfinderAvatarBindings {
    walk_to: Button,
    stop: Button,
}

impl Default for PathfinderAvatarBindings {
    fn default() -> PathfinderAvatarBindings {
        PathfinderAvatarBindings {
            walk_to: Button::Mouse(MouseButton::Right),
            stop: Button::Key(VirtualKeyCode::S),
        }
    }
}

pub struct PathfindingAvatarControls {
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
    pool: ThreadPool,
    world_coord: Option<WorldCoord>,
    bindings: PathfinderAvatarBindings,
}

impl PathfindingAvatarControls {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
        pool: ThreadPool,
    ) -> PathfindingAvatarControls {
        PathfindingAvatarControls {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
            pool,
            bindings: PathfinderAvatarBindings::default(),
            world_coord: None,
        }
    }

    fn walk_to(&mut self) {
        let to = match self.world_coord {
            Some(world_coord) => world_coord.to_v2_round(),
            _ => return,
        };
        let game_tx = self.game_tx.clone();
        let pathfinder_tx = self.pathfinder_tx.clone();
        self.pool.spawn_ok(async move {
            if let Some((name, stop_position, stop_micros)) =
                game_tx.update(move |game| stop_selected_avatar(game)).await
            {
                let positions = pathfinder_tx
                    .update(move |service| service.pathfinder().find_path(&stop_position, &[to]))
                    .await;
                if let Some(positions) = positions {
                    game_tx.update(move |game| {
                        game.walk_positions(name, positions, stop_micros);
                    });
                }
            }
        });
    }

    fn stop(&mut self) {
        self.pool.spawn_ok(self.game_tx.update(move |game| {
            stop_selected_avatar(game);
        }));
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }
}

fn stop_selected_avatar(game: &mut Game) -> Option<(String, V2<usize>, u128)> {
    let (name, state) = match game.game_state().selected_avatar() {
        Some(Avatar { name, state, .. }) => (name, state),
        _ => return None,
    };
    let game_micros = game.game_state().game_micros;
    let new_state = state.stop(&game_micros);
    let out = compute_stop_position_and_micros(&game_micros, &new_state.as_ref().unwrap_or(state))
        .map(|(position, micros)| (name.clone(), position, micros));
    let name = name.clone();
    if let Some(new_state) = new_state {
        game.update_avatar_state(name, new_state)
    }
    out
}

fn compute_stop_position_and_micros(
    game_micros: &u128,
    avatar_state: &AvatarState,
) -> Option<(V2<usize>, u128)> {
    match avatar_state {
        AvatarState::Stationary { position: from, .. } => Some((*from, *game_micros)),
        AvatarState::Walking(path) => {
            let path = path.stop(&game_micros);
            Some((*path.final_position(), *path.final_point_arrival()))
        }
        AvatarState::Absent => None,
    }
}

impl GameEventConsumer for PathfindingAvatarControls {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        }
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.bindings.walk_to {
                self.walk_to();
            } else if button == &self.bindings.stop {
                self.stop();
            };
        }
        CaptureEvent::No
    }
}
