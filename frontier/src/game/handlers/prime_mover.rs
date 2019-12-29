use super::*;
use crate::avatar::*;
use crate::pathfinder::*;
use commons::*;
use isometric::{Button, ElementState, VirtualKeyCode};
use std::collections::HashSet;

pub struct PrimeMover {
    pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    pathfinding_done_tx: Sender<String>,
    pathfinding_done_rx: Receiver<String>,
    pathfinding: HashSet<String>,
    binding: Button,
    active: bool,
}

impl PrimeMover {
    pub fn new(pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>) -> PrimeMover {
        let (pathfinding_done_tx, pathfinding_done_rx) = mpsc::channel();
        PrimeMover {
            pathfinder_tx,
            pathfinding_done_tx,
            pathfinding_done_rx,
            pathfinding: HashSet::new(),
            binding: Button::Key(VirtualKeyCode::K),
            active: false,
        }
    }

    fn move_avatar(
        &mut self,
        game_state: &GameState,
        name: &str,
        avatar_state: &AvatarState,
        farm: &Option<V2<usize>>,
    ) {
        if self.pathfinding.contains(name) {
            return;
        }
        if let (Some(farm), AvatarState::Stationary { position: from, .. }) = (farm, avatar_state) {
            let from = *from;
            let world = &game_state.world;
            let to = if world.get_corners_in_bounds(farm).contains(&from) {
                game_state
                    .territory
                    .who_controls_tile(&game_state.world, &farm)
            } else {
                Some(*farm)
            };

            if let Some(to) = to {
                self.pathfinding.insert(name.to_string());
                let name_string = name.to_string();
                let to = world.get_corners_in_bounds(&to);
                let start_at = game_state.game_micros;
                let pathfinding_done_tx = self.pathfinding_done_tx.clone();
                let function: Box<
                    dyn FnOnce(&Pathfinder<AvatarTravelDuration>) -> Vec<GameCommand> + Send,
                > = Box::new(move |pathfinder| {
                    let result = pathfinder.find_path(&from, &to);
                    pathfinding_done_tx.send(name_string.clone()).unwrap();
                    if let Some(positions) = result {
                        return vec![GameCommand::WalkPositions {
                            name: name_string,
                            positions,
                            start_at,
                        }];
                    } else {
                        return vec![];
                    }
                });
                self.pathfinder_tx
                    .send(PathfinderCommand::Use(function))
                    .unwrap();
            }
        }
    }

    fn move_avatars(&mut self, game_state: &GameState) {
        let selected = game_state.selected_avatar().map(|avatar| &avatar.name);
        for Avatar {
            name, state, farm, ..
        } in game_state.avatars.values()
        {
            if Some(name) != selected {
                self.move_avatar(game_state, name, state, farm);
            }
        }
    }

    fn update_pathfinding_set(&mut self) {
        while let Ok(name) = self.pathfinding_done_rx.try_recv() {
            self.pathfinding.remove(&name);
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
                self.update_pathfinding_set();
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
