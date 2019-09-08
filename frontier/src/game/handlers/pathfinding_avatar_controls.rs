use super::*;
use crate::avatar::*;
use crate::pathfinder::*;
use commons::v2;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, MouseButton, VirtualKeyCode};
use std::default::Default;

pub struct PathfinderAvatarControls {
    walk_to: Button,
    stop: Button,
}

impl Default for PathfinderAvatarControls {
    fn default() -> PathfinderAvatarControls {
        PathfinderAvatarControls {
            walk_to: Button::Mouse(MouseButton::Right),
            stop: Button::Key(VirtualKeyCode::S),
        }
    }
}

pub struct PathfindingAvatarControls {
    command_tx: Sender<GameCommand>,
    pathfinder: Option<Pathfinder<AvatarTravelDuration>>,
    world_coord: Option<WorldCoord>,
    bindings: PathfinderAvatarControls,
}

impl PathfindingAvatarControls {
    pub fn new(command_tx: Sender<GameCommand>) -> PathfindingAvatarControls {
        PathfindingAvatarControls {
            command_tx,
            pathfinder: None,
            world_coord: None,
            bindings: PathfinderAvatarControls::default(),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.pathfinder = Some(Pathfinder::new(
            &game_state.world,
            AvatarTravelDuration::from_params(&game_state.params.avatar_travel),
        ));
    }

    fn reset_pathfinder(&mut self, game_state: &GameState) {
        if let Some(pathfinder) = &mut self.pathfinder {
            pathfinder.compute_network(&game_state.world);
        }
    }

    fn walk_to(&mut self, game_state: &GameState) {
        if let Some(ref pathfinder) = self.pathfinder {
            if let Some(WorldCoord { x, y, .. }) = self.world_coord {
                let to = v2(x.round() as usize, y.round() as usize);
                if let Some(new_state) = game_state.avatar_state.walk_to(
                    &game_state.world,
                    &to,
                    pathfinder,
                    game_state.game_micros,
                ) {
                    self.command_tx
                        .send(GameCommand::UpdateAvatar(new_state))
                        .unwrap();
                }
            }
        }
    }

    fn stop(&mut self, game_state: &GameState) {
        if let Some(new_state) = game_state.avatar_state.stop(&game_state.game_micros) {
            self.command_tx
                .send(GameCommand::UpdateAvatar(new_state))
                .unwrap();
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn update_pathfinder_with_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        if let Some(pathfinder) = &mut self.pathfinder {
            for cell in cells {
                pathfinder.update_node(&game_state.world, cell);
            }
        }
    }

    fn update_pathfinder_with_roads(&mut self, game_state: &GameState, result: &RoadBuilderResult) {
        if let Some(pathfinder) = &mut self.pathfinder {
            result.update_pathfinder(&game_state.world, pathfinder);
        }
    }
}

impl GameEventConsumer for PathfindingAvatarControls {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::CellsRevealed(selection) => {
                match selection {
                    CellSelection::All => self.reset_pathfinder(game_state),
                    CellSelection::Some(cells) => {
                        self.update_pathfinder_with_cells(game_state, &cells)
                    }
                };
            }
            GameEvent::RoadsUpdated(result) => {
                self.update_pathfinder_with_roads(game_state, result)
            }
            _ => (),
        }
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
                self.walk_to(&game_state);
            } else if button == &self.bindings.stop {
                self.stop(&game_state);
            }
        }
        CaptureEvent::No
    }
}
