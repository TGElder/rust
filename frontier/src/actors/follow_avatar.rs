use std::sync::Arc;

use commons::async_trait::async_trait;
use isometric::{Button, ElementState, Event, VirtualKeyCode};

use crate::system::{Capture, HandleEngineEvent};
use crate::traits::has::HasFollowAvatar;

pub struct FollowAvatar<T> {
    cx: T,
    follow_avatar_binding: Button,
}

impl<T> FollowAvatar<T>
where
    T: HasFollowAvatar,
{
    pub fn new(cx: T) -> FollowAvatar<T> {
        FollowAvatar {
            cx,
            follow_avatar_binding: Button::Key(VirtualKeyCode::C),
        }
    }

    async fn toggle_follow_avatar(&mut self) {
        self.cx
            .set_follow_avatar(!self.cx.follow_avatar().await)
            .await
    }
}

#[async_trait]
impl<T> HandleEngineEvent for FollowAvatar<T>
where
    T: HasFollowAvatar + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if button == &self.follow_avatar_binding && !modifiers.alt() {
                self.toggle_follow_avatar().await;
            }
        }
        Capture::No
    }
}
