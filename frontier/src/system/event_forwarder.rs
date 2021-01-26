use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use futures::future::FutureExt;
use isometric::{Event, EventConsumer};
use std::sync::Arc;

use crate::system::Polysender;

pub struct EventForwarderActor {
    tx: Polysender,
}

impl EventForwarderActor {
    pub fn new(tx: Polysender) -> EventForwarderActor {
        EventForwarderActor { tx }
    }
}

impl EventForwarderActor {
    async fn consume_event(&mut self, event: Arc<Event>) {
        if send_event_check_capture(&self.tx.labels_tx, &event).await {
            return;
        }

        send_event(&self.tx.avatar_artist_tx, &event);
        send_event(&self.tx.basic_avatar_controls_tx, &event);
        send_event(&self.tx.basic_road_builder_tx, &event);
        send_event(&self.tx.cheats_tx, &event);
        send_event(&self.tx.object_builder_tx, &event);
        send_event(&self.tx.pathfinding_avatar_controls_tx, &event);
        send_event(&self.tx.rotate_tx, &event);
        send_event(&self.tx.speed_control_tx, &event);
        send_event(&self.tx.town_builder_tx, &event);
        send_event(&self.tx.town_label_artist_tx, &event);
        send_event(&self.tx.world_artist_tx, &event);
    }
}

async fn send_event_check_capture<T>(tx: &FnSender<T>, event: &Arc<Event>) -> bool
where
    T: HandleEngineEvent + Send,
{
    let event = event.clone();
    matches!(
        tx.send_future(|t| t.handle_engine_event(event).boxed())
            .await,
        Capture::Yes
    )
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
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture;
}

pub enum Capture {
    Yes,
    No,
}

pub struct EventForwarderConsumer {
    tx: FnSender<EventForwarderActor>,
}

impl EventForwarderConsumer {
    pub fn new(tx: FnSender<EventForwarderActor>) -> EventForwarderConsumer {
        EventForwarderConsumer { tx }
    }
}

impl EventConsumer for EventForwarderConsumer {
    fn consume_event(&mut self, event: Arc<Event>) {
        self.tx
            .send_future(move |actor| actor.consume_event(event).boxed());
    }
}
