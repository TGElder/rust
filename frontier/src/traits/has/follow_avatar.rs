use commons::async_trait::async_trait;

#[async_trait]
pub trait HasFollowAvatar {
    async fn follow_avatar(&self) -> bool;
    async fn set_follow_avatar(&self, value: bool);
}
