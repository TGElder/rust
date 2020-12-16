use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use commons::futures::future::FutureExt;
use isometric::{Event, EventConsumer};
use std::sync::Arc;

use crate::polysender::Polysender;

pub struct EventForwarder2 {
    x: Polysender,
}

impl EventForwarder2 {
    pub fn new(x: Polysender) -> EventForwarder2 {
        EventForwarder2 { x }
    }
}

impl EventConsumer for EventForwarder2 {
    fn consume_event(&mut self, event: Arc<Event>) {
        send_event(&self.x.basic_road_builder_tx, &event);
        send_event(&self.x.object_builder_tx, &event);
        send_event(&self.x.town_builder_tx, &event);
        send_event(&self.x.town_label_artist_tx, &event);
        send_event(&self.x.world_artist_tx, &event);
    }
}

fn send_event<T>(tx: &FnSender<T>, event: &Arc<Event>)
where
    T: HandleEngineEvent + Send,
{
    let event = event.clone();
    tx.send_future(|t| t.handle_engine_event(event).boxed());
}

#[async_trait]
pub trait HandleEngineEvent {
    async fn handle_engine_event(&mut self, event: Arc<Event>);
}
