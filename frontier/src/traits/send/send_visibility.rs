use futures::future::BoxFuture;

use crate::actors::VisibilityActor;
use crate::traits::has::HasParameters;
use crate::traits::{RevealPositions, WithWorld};

pub trait SendVisibility: HasParameters + RevealPositions + WithWorld + Send + Sync {
    fn send_visibility_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Self>) -> O + Send + 'static;

    fn send_visibility_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Self>) -> BoxFuture<O> + Send + 'static;
}
