use crate::game::{Game, GameState};
use crate::road_builder::{AutoRoadTravelDuration, RoadBuildMode, RoadBuilderResult};
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{SendGame, UpdateRoads};
use crate::travel_duration::TravelDuration;
use crate::world::World;
use commons::async_trait::async_trait;
use commons::edge::Edge;
use commons::V2;
use isometric::{Button, ElementState, Event, VirtualKeyCode};
use std::sync::Arc;

pub struct BasicRoadBuilder<T> {
    tx: T,
    binding: Button,
}

impl<T> BasicRoadBuilder<T>
where
    T: SendGame + UpdateRoads,
{
    pub fn new(tx: T) -> BasicRoadBuilder<T> {
        BasicRoadBuilder {
            tx,
            binding: Button::Key(VirtualKeyCode::R),
        }
    }

    async fn build_road(&mut self) {
        let plan = unwrap_or!(self.get_plan().await, return);
        let result = plan.get_road_builder_result();
        self.walk_positions(plan).await;
        self.update_roads(result).await;
    }

    async fn get_plan(&mut self) -> Option<Plan> {
        self.tx.send_game(|game| get_plan(game)).await
    }

    async fn walk_positions(&mut self, plan: Plan) {
        self.tx
            .send_game(|game| {
                game.walk_positions(plan.avatar_name, plan.forward_path, plan.start_at)
            })
            .await;
    }

    async fn update_roads(&mut self, result: RoadBuilderResult) {
        self.tx.update_roads(result).await;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for BasicRoadBuilder<T>
where
    T: SendGame + UpdateRoads + Send + Sync + 'static,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if *button == self.binding && !modifiers.alt() && modifiers.ctrl() {
                self.build_road().await;
            }
        }
        Capture::No
    }
}

struct Plan {
    avatar_name: String,
    forward_path: Vec<V2<usize>>,
    mode: RoadBuildMode,
    start_at: u128,
}

impl Plan {
    fn get_road_builder_result(&self) -> RoadBuilderResult {
        RoadBuilderResult::new(vec![self.forward_path[0], self.forward_path[1]], self.mode)
    }
}

fn get_plan(game: &Game) -> Option<Plan> {
    let game_state = game.game_state();

    let avatar = game.game_state().avatars.selected()?;
    let forward_path = avatar.path.as_ref()?.forward_path();

    if !is_buildable(game_state, &forward_path) {
        return None;
    }

    Some(Plan {
        avatar_name: avatar.name.clone(),
        mode: get_mode(&game_state.world, &forward_path),
        forward_path,
        start_at: game_state.game_micros,
    })
}

fn is_buildable(game_state: &GameState, forward_path: &[V2<usize>]) -> bool {
    let travel_duration_params = &game_state.params.auto_road_travel;
    let travel_duration = AutoRoadTravelDuration::from_params(travel_duration_params);
    travel_duration
        .get_duration(&game_state.world, &forward_path[0], &forward_path[1])
        .is_some()
}

fn get_mode(world: &World, forward_path: &[V2<usize>]) -> RoadBuildMode {
    let edge = Edge::new(forward_path[0], forward_path[1]);
    if world.is_road(&edge) {
        RoadBuildMode::Demolish
    } else {
        RoadBuildMode::Build
    }
}
