use crate::avatar::{Avatar, AvatarState, AvatarTravelDuration, TravelArgs};
use crate::system::HandleEngineEvent;
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
        let (start_at, selected_avatar) = join!(self.tx.micros(), self.tx.selected_avatar(),);
        let Avatar { name, state, .. } = unwrap_or!(selected_avatar, return);

        let new_state = unwrap_or!(self.get_walk_forward_state(state, start_at).await, return);

        self.tx.update_avatar_state(name, new_state);
    }

    async fn get_walk_forward_state(
        &self,
        state: AvatarState,
        start_at: u128,
    ) -> Option<AvatarState> {
        let path = state.forward_path()?;

        let travel_duration = self.travel_duration.clone();

        self.tx
            .send_world(move |world| {
                travel_duration.get_duration(&world, &path[0], &path[1])?;
                state.travel(TravelArgs {
                    world,
                    positions: path,
                    travel_duration: travel_duration.as_ref(),
                    vehicle_fn: travel_duration.travel_mode_fn(),
                    start_at,
                    pause_at_start: None,
                    pause_at_end: None,
                })
            })
            .await
    }

    async fn rotate_clockwise(&self) {
        if let Some(Avatar { name, state, .. }) = self.tx.selected_avatar().await {
            if let Some(new_state) = state.rotate_clockwise() {
                self.tx.update_avatar_state(name, new_state);
            }
        }
    }

    async fn rotate_anticlockwise(&self) {
        if let Some(Avatar { name, state, .. }) = self.tx.selected_avatar().await {
            if let Some(new_state) = state.rotate_anticlockwise() {
                self.tx.update_avatar_state(name, new_state);
            }
        }
    }
}

#[async_trait]
impl<T> HandleEngineEvent for BasicAvatarControls<T>
where
    T: Micros + SelectedAvatar + SendWorld + UpdateAvatar + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) {
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
    }
}