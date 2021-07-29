use crate::avatar::{Avatar, AvatarTravelDuration, Journey};

use crate::bridges::Bridges;
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{
    BuiltBridges, FindPath, Micros, PathfinderForPlayer, SelectedAvatar, UpdateAvatarJourney,
    WithWorld,
};
use commons::async_trait::async_trait;
use commons::V2;
use isometric::{coords::*, ElementState, Event};
use isometric::{Button, MouseButton, VirtualKeyCode};
use std::default::Default;
use std::sync::Arc;

pub struct PathfindingAvatarControls<T> {
    cx: T,
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
    T: BuiltBridges
        + Micros
        + PathfinderForPlayer
        + SelectedAvatar
        + UpdateAvatarJourney
        + WithWorld,
{
    pub fn new(cx: T, travel_duration: Arc<AvatarTravelDuration>) -> PathfindingAvatarControls<T> {
        PathfindingAvatarControls {
            cx,
            travel_duration,
            bindings: PathfinderAvatarBindings::default(),
            world_coord: None,
        }
    }

    async fn walk_to(&mut self) {
        let to = unwrap_or!(self.world_coord, return).to_v2_round();

        let micros = self.cx.micros().await;

        let (name, journey) = unwrap_or!(self.get_selected_avatar_name_and_journey().await, return);

        let stopped = journey.stop(&micros);
        self.cx
            .update_avatar_journey(&name, Some(stopped.clone()))
            .await;
        let stop_position = stopped.final_frame().position;

        let path = unwrap_or!(
            self.cx
                .player_pathfinder()
                .find_path(&[stop_position], &[to])
                .await,
            return
        );

        let start_at = stopped.final_frame().arrival.max(micros);
        let bridges = self.cx.built_bridges().await;
        let travelling = self
            .extend(stopped, path, start_at, &self.travel_duration, &bridges)
            .await;

        if travelling.is_some() {
            self.cx.update_avatar_journey(&name, travelling).await;
        }
    }

    async fn get_selected_avatar_name_and_journey(&self) -> Option<(String, Journey)> {
        let Avatar { name, journey, .. } = self.cx.selected_avatar().await?;
        let journey = journey?;

        Some((name, journey))
    }

    async fn extend(
        &self,
        journey: Journey,
        positions: Vec<V2<usize>>,
        start_at: u128,
        travel_duration: &AvatarTravelDuration,
        bridges: &Bridges,
    ) -> Option<Journey> {
        self.cx
            .with_world(|world| {
                journey.append(Journey::new(
                    world,
                    positions,
                    travel_duration,
                    travel_duration.travel_mode_fn(),
                    start_at,
                    bridges,
                ))
            })
            .await
    }

    async fn stop(&mut self) {
        let micros = self.cx.micros().await;
        let (name, journey) = unwrap_or!(self.get_selected_avatar_name_and_journey().await, return);

        let stopped = journey.stop(&micros);

        self.cx.update_avatar_journey(&name, Some(stopped)).await;
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for PathfindingAvatarControls<T>
where
    T: BuiltBridges
        + Micros
        + PathfinderForPlayer
        + SelectedAvatar
        + UpdateAvatarJourney
        + WithWorld
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
