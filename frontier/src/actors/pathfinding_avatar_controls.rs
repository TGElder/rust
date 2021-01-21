use crate::avatar::{Avatar, AvatarTravelDuration, Journey};

use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{
    FindPath, Micros, PathfinderWithoutPlannedRoads, SelectedAvatar, SendWorld, UpdateAvatarJourney,
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
    T: Micros + PathfinderWithoutPlannedRoads + SelectedAvatar + SendWorld + UpdateAvatarJourney,
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

        let (name, journey) = unwrap_or!(self.get_selected_avatar_name_and_journey().await, return);

        let stopped = journey.stop(&micros);
        let stop_position = stopped.final_frame().position;

        let path = unwrap_or!(
            self.tx
                .pathfinder_without_planned_roads()
                .find_path(vec![stop_position], vec![to])
                .await,
            return
        );

        let start_at = stopped.final_frame().arrival.max(micros);
        let travelling = self
            .extend(stopped, path, start_at, self.travel_duration.clone())
            .await;

        if travelling.is_some() {
            self.tx.update_avatar_journey(name, travelling).await;
        }
    }

    async fn get_selected_avatar_name_and_journey(&self) -> Option<(String, Journey)> {
        let Avatar { name, journey, .. } = self.tx.selected_avatar().await?;
        let journey = journey?;

        Some((name, journey))
    }

    async fn extend(
        &self,
        journey: Journey,
        positions: Vec<V2<usize>>,
        start_at: u128,
        travel_duration: Arc<AvatarTravelDuration>,
    ) -> Option<Journey> {
        self.tx
            .send_world(move |world| {
                journey.extend(
                    world,
                    positions,
                    travel_duration.as_ref(),
                    travel_duration.travel_mode_fn(),
                    start_at,
                )
            })
            .await
    }

    async fn stop(&mut self) {
        let micros = self.tx.micros().await;
        let (name, journey) = unwrap_or!(self.get_selected_avatar_name_and_journey().await, return);

        let stopped = journey.stop(&micros);

        self.tx.update_avatar_journey(name, Some(stopped)).await;
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for PathfindingAvatarControls<T>
where
    T: Micros
        + PathfinderWithoutPlannedRoads
        + SelectedAvatar
        + SendWorld
        + UpdateAvatarJourney
        + Send
        + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
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
        Capture::No
    }
}
