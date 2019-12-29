use super::*;
use crate::pathfinder::*;
use crate::road_builder::*;
use commons::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

pub struct PathfindingRoadBuilder {
    pathfinder_tx: Sender<PathfinderCommand<AutoRoadTravelDuration>>,
    binding: Button,
    world_coord: Option<WorldCoord>,
}

impl PathfindingRoadBuilder {
    pub fn new(
        pathfinder_tx: Sender<PathfinderCommand<AutoRoadTravelDuration>>,
    ) -> PathfindingRoadBuilder {
        PathfindingRoadBuilder {
            pathfinder_tx,
            binding: Button::Key(VirtualKeyCode::X),
            world_coord: None,
        }
    }

    fn walk_forward(&mut self, game_state: &GameState) {
        if let Some(Avatar {
            state: AvatarState::Stationary { position: from, .. },
            ..
        }) = game_state.selected_avatar()
        {
            if let Some(WorldCoord { x, y, .. }) = self.world_coord {
                let from = *from;
                let to = v2(x.round() as usize, y.round() as usize);
                let function: Box<
                    dyn FnOnce(&Pathfinder<AutoRoadTravelDuration>) -> Vec<GameCommand> + Send,
                > = Box::new(move |pathfinder| {
                    if let Some(result) = auto_build_road(from, to, &pathfinder) {
                        return vec![GameCommand::UpdateRoads(result)];
                    }
                    vec![]
                });
                self.pathfinder_tx
                    .send(PathfinderCommand::Use(function))
                    .unwrap();
            }
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }
}

impl GameEventConsumer for PathfindingRoadBuilder {
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
            if button == &self.binding {
                self.walk_forward(game_state);
            }
        }
        CaptureEvent::No
    }
}
