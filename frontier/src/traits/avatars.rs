use commons::async_trait::async_trait;

use crate::avatar::{Avatar, Journey};
use crate::traits::WithAvatars;

#[async_trait]
pub trait SelectedAvatar {
    async fn selected_avatar(&self) -> Option<Avatar>;
}

#[async_trait]
impl<T> SelectedAvatar for T
where
    T: WithAvatars + Send + Sync,
{
    async fn selected_avatar(&self) -> Option<Avatar> {
        self.with_avatars(|avatars| avatars.selected().cloned())
            .await
    }
}

#[async_trait]
pub trait UpdateAvatarJourney {
    async fn update_avatar_journey(&self, name: &str, journey: Option<Journey>);
}

#[async_trait]
impl<T> UpdateAvatarJourney for T
where
    T: WithAvatars + Send + Sync,
{
    async fn update_avatar_journey(&self, name: &str, journey: Option<Journey>) {
        self.mut_avatars(|avatars| {
            if let Some(avatar) = avatars.all.get_mut(name) {
                avatar.journey = journey;
            }
        })
        .await
    }
}
