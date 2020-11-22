use super::*;

use commons::async_trait::async_trait;
use commons::V2;
use std::collections::HashSet;

#[async_trait]
pub trait Visibility {
    fn check_visibility_and_reveal(&mut self, visited: HashSet<V2<usize>>);
    fn disable_visibility_computation(&mut self);
}

#[async_trait]
impl<T> Visibility for T
where
    T: SendVisibility,
{
    fn check_visibility_and_reveal(&mut self, visited: HashSet<V2<usize>>) {
        self.send_visibility_background(move |visibility| {
            visibility.check_visibility_and_reveal(visited)
        });
    }

    fn disable_visibility_computation(&mut self) {
        self.send_visibility_background(move |visibility| {
            visibility.disable_visibility_computation()
        });
    }
}
