use super::*;
use crate::avatar::*;
use crate::pathfinder::*;
use commons::v2;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, MouseButton, VirtualKeyCode};
use std::default::Default;

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
    command_tx: Sender<GameCommand>,
    pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    bindings: PathfinderAvatarBindings,
    world_coord: Option<WorldCoord>,
}

impl PathfindingAvatarControls {
    pub fn new(
        command_tx: Sender<GameCommand>,
        pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    ) -> PathfindingAvatarControls {
        PathfindingAvatarControls {
            command_tx,
            pathfinder_tx,
            bindings: PathfinderAvatarBindings::default(),
            world_coord: None,
        }
    }

    fn compute_from_and_start_at(
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

    fn walk_to(&mut self, game_state: &GameState) {
        if let Some(WorldCoord { x, y, .. }) = self.world_coord {
            if let Some((name, avatar_state)) = game_state.selected_avatar_name_and_state() {
                let name = name.to_string();
                let to = v2(x.round() as usize, y.round() as usize);
                if let Some((from, start_at)) =
                    Self::compute_from_and_start_at(&game_state.game_micros, &avatar_state)
                {
                    self.stop(&game_state);
                    let function: Box<
                        dyn Fn(&Pathfinder<AvatarTravelDuration>) -> Vec<GameCommand> + Send,
                    > = Box::new(move |pathfinder| {
                        if let Some(positions) = pathfinder.find_path(&from, &to) {
                            return vec![GameCommand::WalkPositions {
                                name: name.clone(),
                                positions,
                                start_at,
                            }];
                        }
                        vec![]
                    });
                    self.pathfinder_tx
                        .send(PathfinderCommand::Use(function))
                        .unwrap();
                }
            }
        }
    }

    fn stop(&mut self, game_state: &GameState) {
        if let Some((name, avatar_state)) = game_state.selected_avatar_name_and_state() {
            if let Some(new_state) = avatar_state.stop(&game_state.game_micros) {
                self.command_tx
                    .send(GameCommand::UpdateAvatar {
                        name: name.to_string(),
                        new_state,
                    })
                    .unwrap();
            }
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }
}

impl GameEventConsumer for PathfindingAvatarControls {
    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
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
                self.walk_to(&game_state)
            } else if button == &self.bindings.stop {
                self.stop(&game_state)
            };
        }
        CaptureEvent::No
    }
}
