use commons::future::BoxFuture;

use crate::actors::VisibilityActor;
use crate::traits::{RevealPositions, SendGame, SendWorld};

pub trait SendVisibility: RevealPositions + SendGame + SendWorld + Send {
    fn send_visibility_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Self>) -> O + Send + 'static;

    fn send_visibility_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Self>) -> BoxFuture<O> + Send + 'static;
}
