use crate::polysender::traits::Parallel;

use super::*;

use commons::async_trait::async_trait;

#[async_trait]
pub trait WithVisibility {
    async fn with_visibility<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static;

    fn with_visibility_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static;
}

#[async_trait]
pub trait Visibility {
    fn check_visibility_and_reveal(&mut self, visited: HashSet<V2<usize>>);
    fn deactive_visibility(&mut self);
}

#[async_trait]
impl<T> Visibility for T
where
    T: Parallel + WithVisibility,
{
    fn check_visibility_and_reveal(&mut self, visited: HashSet<V2<usize>>) {
        self.with_visibility_background(move |visibility| {
            visibility.check_visibility_and_reveal(visited)
        });
    }

    fn deactive_visibility(&mut self) {
        self.with_visibility_background(move |visibility| visibility.deactive());
    }
}
