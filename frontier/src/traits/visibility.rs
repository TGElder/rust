use crate::traits::RevealPositions;
use crate::visited::Visited;

use super::*;

use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::V2;
use std::collections::HashSet;

const NAME: &str = "visibility_trait";

#[async_trait]
pub trait Visibility {
    async fn check_visibility_and_reveal(&self, visited: HashSet<V2<usize>>);
}

#[async_trait]
impl<T> Visibility for T
where
    T: RevealPositions + WithVisibility + WithVisited + Send + Sync,
{
    async fn check_visibility_and_reveal(&self, visited: HashSet<V2<usize>>) {
        if self.with_visited(|visited| visited.all_visited).await {
            return;
        }

        let newly_visited = get_newly_visited(self, visited).await;
        if newly_visited.is_empty() {
            return;
        }

        let (_, visible) = join!(
            set_visited(self, &newly_visited),
            get_visible(self, &newly_visited),
        );
        if visible.is_empty() {
            return;
        }

        self.reveal_positions(&visible, NAME).await;
    }
}

async fn get_newly_visited<T>(cx: &T, mut visited: HashSet<V2<usize>>) -> HashSet<V2<usize>>
where
    T: WithVisited,
{
    cx.with_visited(|Visited { positions, .. }| {
        visited.retain(|position| !positions.get_cell_unsafe(position))
    })
    .await;
    visited
}

async fn set_visited<T>(cx: &T, visited: &HashSet<V2<usize>>)
where
    T: WithVisited,
{
    cx.mut_visited(|Visited { positions, .. }| {
        for position in visited.iter() {
            *positions.mut_cell_unsafe(position) = true;
        }
    })
    .await;
}

async fn get_visible<T>(cx: &T, visited: &HashSet<V2<usize>>) -> HashSet<V2<usize>>
where
    T: WithVisibility,
{
    cx.with_visibility(|visibility| {
        visited
            .iter()
            .flat_map(|position| visibility.get_visible_from(*position))
            .collect::<HashSet<_>>()
    })
    .await
}
