use futures::future::BoxFuture;

use crate::actors::Voyager;
use crate::traits::{RevealPositions, SendSettlements, WithWorld};

pub trait SendVoyager: RevealPositions + SendSettlements + WithWorld + Send + Sync {
    fn send_voyager_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Voyager<Self>) -> BoxFuture<O> + Send + 'static;
}
