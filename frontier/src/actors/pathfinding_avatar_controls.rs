use crate::avatar::{Avatar, AvatarState, AvatarTravelDuration, TravelArgs};
use crate::system::HandleEngineEvent;
use crate::traits::{
    FindPath, Micros, PathfinderWithoutPlannedRoads, SelectedAvatar, SendWorld, UpdateAvatar,
};
use commons::async_trait::async_trait;
use commons::V2;
use isometric::{coords::*, ElementState, Event};
use isometric::{Button, MouseButton, VirtualKeyCode};
use std::default::Default;
use std::sync::Arc;

pub struct PathfindingAvatarControls<T> {
    tx: T,
    travel_duration: Arc<AvatarTravelDuration>,
    world_coord: Option<WorldCoord>,
    bindings: PathfinderAvatarBindings,
}

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

impl<T> PathfindingAvatarControls<T>
where
    T: Micros + PathfinderWithoutPlannedRoads + SelectedAvatar + SendWorld + UpdateAvatar,
{
    pub fn new(tx: T, travel_duration: Arc<AvatarTravelDuration>) -> PathfindingAvatarControls<T> {
        PathfindingAvatarControls {
            tx,
            travel_duration,
            bindings: PathfinderAvatarBindings::default(),
            world_coord: None,
        }
    }

    async fn walk_to(&mut self) {
        let to = unwrap_or!(self.world_coord, return).to_v2_round();

        let micros = self.tx.micros().await;

        let (name, state) = unwrap_or!(self.stop_selected_avatar(&micros).await, return);

        let stop_position = *unwrap_or!(stop_position(&state), return);

        let path = unwrap_or!(
            self.tx
                .pathfinder_without_planned_roads()
                .find_path(vec![stop_position], vec![to])
                .await,
            return
        );

        let start_at = *stop_micros(&state).unwrap_or(&micros);

        let travelling = self
            .travel(state, path, start_at, self.travel_duration.clone())
            .await;

        if let Some(travelling) = travelling {
            self.tx.update_avatar_state(name, travelling).await;
        }
    }

    async fn stop_selected_avatar(&self, micros: &u128) -> Option<(String, AvatarState)> {
        let Avatar { name, state, .. } = self.tx.selected_avatar().await?;

        let stopped = state.stop(&micros);

        if let Some(stopped) = &stopped {
            self.tx
                .update_avatar_state(name.clone(), stopped.clone())
                .await;
        }

        Some((name, stopped.unwrap_or(state)))
    }

    async fn travel(
        &self,
        state: AvatarState,
        positions: Vec<V2<usize>>,
        start_at: u128,
        travel_duration: Arc<AvatarTravelDuration>,
    ) -> Option<AvatarState> {
        self.tx
            .send_world(move |world| {
                state.travel(TravelArgs {
                    world: &world,
                    positions,
                    travel_duration: travel_duration.as_ref(),
                    vehicle_fn: travel_duration.travel_mode_fn(),
                    start_at,
                    pause_at_start: None,
                    pause_at_end: None,
                })
            })
            .await
    }

    async fn stop(&mut self) {
        self.stop_selected_avatar(&self.tx.micros().await).await;
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }
}

fn stop_position(avatar_state: &AvatarState) -> Option<&V2<usize>> {
    match avatar_state {
        AvatarState::Stationary { position, .. } => Some(position),
        AvatarState::Walking(path) => Some(&path.final_frame().position),
        AvatarState::Absent => None,
    }
}

fn stop_micros(avatar_state: &AvatarState) -> Option<&u128> {
    match avatar_state {
        AvatarState::Stationary { .. } => None,
        AvatarState::Walking(path) => Some(&path.final_frame().arrival),
        AvatarState::Absent => None,
    }
}

#[async_trait]
impl<T> HandleEngineEvent for PathfindingAvatarControls<T>
where
    T: Micros
        + PathfinderWithoutPlannedRoads
        + SelectedAvatar
        + SendWorld
        + UpdateAvatar
        + Send
        + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) {
        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        }
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if button == &self.bindings.walk_to && !modifiers.alt() {
                self.walk_to().await;
            } else if button == &self.bindings.stop && !modifiers.alt() {
                self.stop().await;
            };
        }
    }
}
