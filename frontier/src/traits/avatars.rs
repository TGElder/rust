use commons::async_trait::async_trait;

use crate::avatar::{Avatar, Path};
use crate::traits::SendAvatars;

#[async_trait]
pub trait SelectedAvatar {
    async fn selected_avatar(&self) -> Option<Avatar>;
}

#[async_trait]
impl<T> SelectedAvatar for T
where
    T: SendAvatars + Send + Sync,
{
    async fn selected_avatar(&self) -> Option<Avatar> {
        self.send_avatars(|avatars| avatars.selected().cloned())
            .await
    }
}

#[async_trait]
pub trait UpdateAvatar {
    async fn update_avatar_path(&self, name: String, path: Option<Path>);
}

#[async_trait]
impl<T> UpdateAvatar for T
where
    T: SendAvatars + Send + Sync,
{
    async fn update_avatar_path(&self, name: String, path: Option<Path>) {
        self.send_avatars(move |avatars| {
            if let Some(avatar) = avatars.all.get_mut(&name) {
                avatar.path = path;
            }
        })
        .await
    }
}
