use crate::actors::VisibilityActor;
use crate::traits::{SendGame, SendWorld};

pub trait SendVisibility: SendGame + SendWorld + Send {
    fn send_visibility_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor<Self>) -> O + Send + 'static;
}
