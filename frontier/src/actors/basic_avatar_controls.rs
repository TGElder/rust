use crate::avatar::{Avatar, AvatarTravelDuration, Journey};
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{Micros, SelectedAvatar, UpdateAvatarJourney, WithWorld};
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
    T: Micros + SelectedAvatar + UpdateAvatarJourney + WithWorld + Send + Sync,
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
        let Avatar { name, journey, .. } = unwrap_or!(selected_avatar, return);
        let journey = unwrap_or!(journey, return);

        let stopped = journey.stop(&micros);
        let start_at = stopped.final_frame().arrival.max(micros);
        let new_journey = unwrap_or!(
            self.get_walk_forward_journey(stopped, start_at).await,
            return
        );

        self.tx.update_avatar_journey(name, Some(new_journey)).await;
    }

    async fn get_walk_forward_journey(&self, journey: Journey, start_at: u128) -> Option<Journey> {
        let positions = journey.forward_path();

        self.tx
            .with_world(|world| {
                self.travel_duration
                    .get_duration(&world, &positions[0], &positions[1])?;
                journey.append(Journey::new(
                    world,
                    positions,
                    self.travel_duration.as_ref(),
                    self.travel_duration.travel_mode_fn(),
                    start_at,
                ))
            })
            .await
    }

    async fn rotate_clockwise(&self) {
        if let Some(Avatar { name, journey, .. }) = self.tx.selected_avatar().await {
            if let Some(journey) = journey {
                self.tx
                    .update_avatar_journey(name, Some(journey.then_rotate_clockwise()))
                    .await;
            }
        }
    }

    async fn rotate_anticlockwise(&self) {
        if let Some(Avatar { name, journey, .. }) = self.tx.selected_avatar().await {
            if let Some(journey) = journey {
                self.tx
                    .update_avatar_journey(name, Some(journey.then_rotate_anticlockwise()))
                    .await;
            }
        }
    }
}

#[async_trait]
impl<T> HandleEngineEvent for BasicAvatarControls<T>
where
    T: Micros + SelectedAvatar + UpdateAvatarJourney + WithWorld + Send + Sync,
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
