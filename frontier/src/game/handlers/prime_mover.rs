use super::*;
use crate::avatar::*;
use crate::pathfinder::*;
use commons::v2;
use commons::*;
use isometric::{Button, ElementState, VirtualKeyCode};
use rand::prelude::*;

pub struct PrimeMover {
    command_tx: Sender<GameCommand>,
    pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    binding: Button,
    active: bool,
}

impl PrimeMover {
    pub fn new(
        command_tx: Sender<GameCommand>,
        pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    ) -> PrimeMover {
        PrimeMover {
            command_tx,
            pathfinder_tx,
            binding: Button::Key(VirtualKeyCode::K),
            active: false,
        }
    }

    fn random_location(&self, world: &World) -> V2<usize> {
        loop {
            let mut rng = rand::thread_rng();
            let x = rng.gen_range(0, world.width());
            let y = rng.gen_range(0, world.height());
            let position = v2(x, y);
            if !world.is_sea(&position) {
                return position;
            }
        }
    }

    fn move_avatar(&mut self, game_state: &GameState, name: &str, avatar_state: &AvatarState) {
        if let AvatarState::Stationary {
            position: from,
            rotation,
            thinking: false,
        } = avatar_state
        {
            let name_string = name.to_string();
            let from = *from;
            let to = self.random_location(&game_state.world);
            let rotation = *rotation;
            let start_at = game_state.game_micros;
            let function: Box<
                dyn Fn(&Pathfinder<AvatarTravelDuration>) -> Vec<GameCommand> + Send,
            > = Box::new(move |pathfinder| {
                if let Some(positions) = pathfinder.find_path(&from, &to) {
                    return vec![GameCommand::WalkPositions {
                        name: name_string.clone(),
                        positions,
                        start_at,
                    }];
                } else {
                    return vec![GameCommand::UpdateAvatar {
                        name: name_string.clone(),
                        new_state: AvatarState::Stationary {
                            position: from,
                            rotation,
                            thinking: false,
                        },
                    }];
                }
            });
            self.command_tx
                .send(GameCommand::UpdateAvatar {
                    name: name.to_string(),
                    new_state: AvatarState::Stationary {
                        position: from,
                        rotation,
                        thinking: true,
                    },
                })
                .unwrap();
            self.pathfinder_tx
                .send(PathfinderCommand::Use(function))
                .unwrap();
        }
    }

    fn move_avatars(&mut self, game_state: &GameState) {
        for (name, avatar_state) in game_state.avatar_state.iter() {
            self.move_avatar(game_state, name, avatar_state);
        }
    }
}

impl GameEventConsumer for PrimeMover {
    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Tick = *event {
            if self.active {
                self.move_avatars(game_state);
            }
        } else if let Event::Button {
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
