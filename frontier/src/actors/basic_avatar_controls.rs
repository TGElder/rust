use crate::avatar::{Avatar, AvatarTravelDuration, Path};
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{Micros, SelectedAvatar, SendWorld, UpdateAvatar};
use crate::travel_duration::TravelDuration;
use commons::async_trait::async_trait;
use isometric::{Button, ElementState, Event, VirtualKeyCode};
use std::default::Default;
use std::sync::Arc;

pub struct BasicAvatarControls<T> {
    tx: T,
    travel_duration: Arc<AvatarTravelDuration>,
    bindings: BasicAvatarBindings,
}
pub struct BasicAvatarBindings {
    forward: Button,
    rotate_clockwise: Button,
    rotate_anticlockwise: Button,
}

impl Default for BasicAvatarBindings {
    fn default() -> BasicAvatarBindings {
        BasicAvatarBindings {
            forward: Button::Key(VirtualKeyCode::W),
            rotate_clockwise: Button::Key(VirtualKeyCode::D),
            rotate_anticlockwise: Button::Key(VirtualKeyCode::A),
        }
    }
}

impl<T> BasicAvatarControls<T>
where
    T: Micros + SelectedAvatar + SendWorld + UpdateAvatar,
{
    pub fn new(tx: T, travel_duration: Arc<AvatarTravelDuration>) -> BasicAvatarControls<T> {
        BasicAvatarControls {
            tx,
            travel_duration,
            bindings: BasicAvatarBindings::default(),
        }
    }

    async fn walk_forward(&self) {
        let (micros, selected_avatar) = join!(self.tx.micros(), self.tx.selected_avatar(),);
        let Avatar { name, path, .. } = unwrap_or!(selected_avatar, return);
        let path = unwrap_or!(path, return);

        let stopped = path.stop(&micros);
        let start_at = stopped.final_frame().arrival.max(micros);
        let new_path = unwrap_or!(self.get_walk_forward_path(stopped, start_at).await, return);

        self.tx.update_avatar_path(name, Some(new_path)).await;
    }

    async fn get_walk_forward_path(&self, path: Path, start_at: u128) -> Option<Path> {
        let positions = path.forward_path();

        let travel_duration = self.travel_duration.clone();

        self.tx
            .send_world(move |world| {
                travel_duration.get_duration(&world, &positions[0], &positions[1])?;
                path.extend(
                    world,
                    positions,
                    travel_duration.as_ref(),
                    travel_duration.travel_mode_fn(),
                    start_at,
                )
            })
            .await
    }

    async fn rotate_clockwise(&self) {
        if let Some(Avatar { name, path, .. }) = self.tx.selected_avatar().await {
            if let Some(path) = path {
                self.tx
                    .update_avatar_path(name, Some(path.then_rotate_clockwise()))
                    .await;
            }
        }
    }

    async fn rotate_anticlockwise(&self) {
        if let Some(Avatar { name, path, .. }) = self.tx.selected_avatar().await {
            if let Some(path) = path {
                self.tx
                    .update_avatar_path(name, Some(path.then_rotate_anticlockwise()))
                    .await;
            }
        }
    }
}

#[async_trait]
impl<T> HandleEngineEvent for BasicAvatarControls<T>
where
    T: Micros + SelectedAvatar + SendWorld + UpdateAvatar + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if button == &self.bindings.forward && !modifiers.alt() {
                self.walk_forward().await;
            } else if button == &self.bindings.rotate_clockwise && !modifiers.alt() {
                self.rotate_clockwise().await;
            } else if button == &self.bindings.rotate_anticlockwise && !modifiers.alt() {
                self.rotate_anticlockwise().await;
            };
        }
        Capture::No
    }
}
