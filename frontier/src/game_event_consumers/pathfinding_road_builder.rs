use super::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "pathfinding_road_builder";

pub struct PathfindingRoadBuilder {
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<PathfinderService<AutoRoadTravelDuration>>,
    pool: ThreadPool,
    world_coord: Option<WorldCoord>,
    binding: Button,
}

impl PathfindingRoadBuilder {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<PathfinderService<AutoRoadTravelDuration>>,
        pool: ThreadPool,
    ) -> PathfindingRoadBuilder {
        PathfindingRoadBuilder {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
            pool,
            world_coord: None,
            binding: Button::Key(VirtualKeyCode::X),
        }
    }

    fn build_road(&mut self, game_state: &GameState) {
        let from = *match game_state.selected_avatar() {
            Some(Avatar {
                state: AvatarState::Stationary { position: from, .. },
                ..
            }) => from,
            _ => return,
        };
        let to = unwrap_or!(self.world_coord, return).to_v2_round();
        let pathfinder_tx = self.pathfinder_tx.clone();
        let game_tx = self.game_tx.clone();
        self.pool.spawn_ok(async move {
            let result = pathfinder_tx
                .update(move |service| auto_build_road(from, to, &service.pathfinder()))
                .await;
            if let Some(result) = result {
                game_tx.update(move |game| game.update_roads(result));
            }
        });
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }
}

impl GameEventConsumer for PathfindingRoadBuilder {
    fn name(&self) -> &'static str {
        HANDLE
    }

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
                self.build_road(game_state);
            }
        }
        CaptureEvent::No
    }
}
