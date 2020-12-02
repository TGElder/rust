use crate::game::{Game, GameState};
use crate::road_builder::{AutoRoadTravelDuration, RoadBuildMode, RoadBuilderResult};
use crate::traits::{SendGame, UpdateRoads};
use crate::travel_duration::TravelDuration;
use crate::world::World;
use commons::async_channel::{Receiver, RecvError};
use commons::edge::Edge;
use commons::futures::future::FutureExt;
use commons::V2;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

pub struct BasicRoadBuilder<T> {
    x: T,
    engine_rx: Receiver<Arc<Event>>,
    binding: Button,
    run: bool,
}

impl<T> BasicRoadBuilder<T>
where
    T: SendGame + UpdateRoads,
{
    pub fn new(x: T, engine_rx: Receiver<Arc<Event>>) -> BasicRoadBuilder<T> {
        BasicRoadBuilder {
            x,
            engine_rx,
            binding: Button::Key(VirtualKeyCode::R),
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        let event: Arc<Event> = event.unwrap();
        match *event {
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers: ModifiersState { alt: false, .. },
                ..
            } if *button == self.binding => self.build_road().await,
            Event::Shutdown => self.shutdown(),
            _ => (),
        }
    }

    async fn build_road(&mut self) {
        let plan = unwrap_or!(self.get_plan().await, return);
        let result = plan.get_road_builder_result();
        self.walk_positions(plan).await;
        self.update_roads(result).await;
    }

    async fn get_plan(&mut self) -> Option<Plan> {
        self.x.send_game(|game| get_plan(game)).await
    }

    async fn walk_positions(&mut self, plan: Plan) {
        self.x
            .send_game(|game| {
                game.walk_positions(
                    plan.avatar_name,
                    plan.forward_path,
                    plan.start_at,
                    None,
                    None,
                )
            })
            .await;
    }

    async fn update_roads(&mut self, result: RoadBuilderResult) {
        self.x.update_roads(result).await;
    }

    fn shutdown(&mut self) {
        self.run = false;
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

    let avatar = game.game_state().selected_avatar()?;
    let forward_path = avatar.state.forward_path()?;

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
