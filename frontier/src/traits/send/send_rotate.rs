use crate::actors::Rotate;

pub trait SendRotate {
    fn send_rotate_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Rotate) -> O + Send + 'static;
}
