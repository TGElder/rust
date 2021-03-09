use crate::traits::RevealPositions;

use super::*;

use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::log::debug;
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
    T: WithVisibility + WithVisited + RevealPositions + Send + Sync,
{
    async fn check_visibility_and_reveal(&self, visited: HashSet<V2<usize>>) {
        if self.with_visited(|visited| visited.all_visited).await {
            return;
        }

        let mut newly_visited = visited;

        self.with_visited(|visited| {
            newly_visited.retain(|position| !visited.visited.get_cell_unsafe(position))
        })
        .await;

        if newly_visited.is_empty() {
            return;
        }

        debug!("Visiting {:?}", newly_visited);

        self.mut_visited(|visited| {
            for position in newly_visited.iter() {
                *visited.visited.mut_cell_unsafe(position) = true;
            }
        }).await;

        let visible = self
            .with_visibility(|visibility| {
                newly_visited
                    .into_iter()
                    .flat_map(|position| visibility.get_visible_from(position))
                    .collect::<HashSet<_>>()
            })
            .await;

        if visible.is_empty() {
            return;
        }

        self.reveal_positions(&visible, NAME).await;
    }
}
