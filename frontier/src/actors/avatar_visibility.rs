use crate::traits::{Micros, SelectedAvatar, Visibility};
use commons::async_std::task::sleep;
use commons::async_trait::async_trait;
use commons::process::Step;
use commons::V2;
use std::collections::HashSet;
use std::time::Duration;

pub struct AvatarVisibility<T> {
    tx: T,
    last_update: Option<u128>,
    refresh_interval: Duration,
}

impl<T> AvatarVisibility<T>
where
    T: Micros + SelectedAvatar + Visibility,
{
    pub fn new(tx: T) -> AvatarVisibility<T> {
        AvatarVisibility {
            tx,
            last_update: None,
            refresh_interval: Duration::from_millis(100),
        }
    }

    async fn get_visited(&self, from: &Option<u128>, to: &u128) -> Option<HashSet<V2<usize>>> {
        let journey = self.tx.selected_avatar().await?.journey?;
        Some(
            journey
                .frames_between_times(&from.unwrap_or(0), to)
                .iter()
                .map(|frame| frame.position)
                .collect(),
        )
    }
}

#[async_trait]
impl<T> Step for AvatarVisibility<T>
where
    T: Micros + SelectedAvatar + Visibility + Send + Sync,
{
    async fn step(&mut self) {
        let until = self.tx.micros().await;
        if let Some(visited) = self.get_visited(&self.last_update, &until).await {
            self.tx.check_visibility_and_reveal(visited);
        }
        self.last_update = Some(until);
        sleep(self.refresh_interval).await;
    }
}
