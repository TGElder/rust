use crate::actors::VisibilityActor;

pub trait SendVisibility {
    fn send_visibility_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static;
}
