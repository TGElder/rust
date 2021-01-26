use futures::future::BoxFuture;

use crate::actors::VisibilityActor;
use crate::traits::{RevealPositions, SendParameters, SendWorld};

pub trait SendVisibility: RevealPositions + SendParameters + SendWorld + Send + Sync {
    fn send_visibility_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Self>) -> O + Send + 'static;

    fn send_visibility_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Self>) -> BoxFuture<O> + Send + 'static;
}
