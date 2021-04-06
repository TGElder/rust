use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use futures::future::FutureExt;
use isometric::{Event, EventConsumer};
use std::sync::Arc;

use crate::system::Context;

pub struct EventForwarderActor {
    cx: Context,
}

impl EventForwarderActor {
    pub fn new(cx: Context) -> EventForwarderActor {
        EventForwarderActor { cx }
    }
}

impl EventForwarderActor {
    async fn consume_event(&mut self, event: Arc<Event>) {
        if send_event_check_capture(&self.cx.labels_tx, &event).await {
            return;
        }

        send_event(&self.cx.basic_avatar_controls_tx, &event);
        send_event(&self.cx.basic_road_builder_tx, &event);
        send_event(&self.cx.cheats_tx, &event);
        send_event(&self.cx.follow_avatar_tx, &event);
        send_event(&self.cx.object_builder_tx, &event);
        send_event(&self.cx.pathfinding_avatar_controls_tx, &event);
        send_event(&self.cx.rotate_tx, &event);
        send_event(&self.cx.speed_control_tx, &event);
        send_event(&self.cx.town_builder_tx, &event);
        send_event(&self.cx.town_label_artist_tx, &event);
        send_event(&self.cx.world_artist_tx, &event);
    }
}

async fn send_event_check_capture<T>(cx: &FnSender<T>, event: &Arc<Event>) -> bool
where
    T: HandleEngineEvent + Send,
{
    let event = event.clone();
    matches!(
        cx.send_future(|t| t.handle_engine_event(event).boxed())
            .await,
        Capture::Yes
    )
}

fn send_event<T>(cx: &FnSender<T>, event: &Arc<Event>)
where
    T: HandleEngineEvent + Send,
{
    let event = event.clone();
    cx.send_future(|t| t.handle_engine_event(event).boxed());
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
    cx: FnSender<EventForwarderActor>,
}

impl EventForwarderConsumer {
    pub fn new(cx: FnSender<EventForwarderActor>) -> EventForwarderConsumer {
        EventForwarderConsumer { cx }
    }
}

impl EventConsumer for EventForwarderConsumer {
    fn consume_event(&mut self, event: Arc<Event>) {
        self.cx
            .send_future(move |actor| actor.consume_event(event).boxed());
    }
}
