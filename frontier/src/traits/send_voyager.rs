use commons::future::BoxFuture;

use crate::actors::Voyager;
use crate::traits::{RevealCells, SendGame, SendWorld};

pub trait SendVoyager: RevealCells + SendGame + SendWorld + Send {
    fn send_voyager_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Voyager<Self>) -> BoxFuture<O> + Send + 'static;
}
