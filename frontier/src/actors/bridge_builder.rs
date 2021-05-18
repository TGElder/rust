use crate::bridge::Bridge;
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::AddBridge;
use commons::async_trait::async_trait;
use commons::edge::Edge;
use commons::V2;
use isometric::coords::WorldCoord;
use isometric::{Button, ElementState, Event, VirtualKeyCode};
use std::sync::Arc;
use std::time::Duration;

pub struct BridgeBuilderActor<T> {
    cx: T,
    binding: Button,
    from: Option<V2<usize>>,
    world_coord: Option<WorldCoord>,
    bridge_duration_millis: u64,
}

impl<T> BridgeBuilderActor<T>
where
    T: AddBridge,
{
    pub fn new(cx: T, bridge_duration_millis: u64) -> BridgeBuilderActor<T> {
        BridgeBuilderActor {
            cx,
            binding: Button::Key(VirtualKeyCode::G),
            from: None,
            world_coord: None,
            bridge_duration_millis,
        }
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    async fn build_bridge(&mut self) {
        let world_coord = unwrap_or!(self.world_coord, return);
        let position = world_coord.to_v2_round();
        match self.from.take() {
            Some(from) => self.complete_bridge(from, position).await,
            None => self.from = Some(position),
        }
    }

    async fn complete_bridge(&mut self, from: V2<usize>, to: V2<usize>) {
        let edge = Edge::new(from, to);
        let bridge = Bridge {
            duration: Duration::from_millis(self.bridge_duration_millis) * (edge.length() as u32),
            edge,
            vehicle: crate::avatar::Vehicle::None,
        };
        self.cx.add_bridge(bridge).await;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for BridgeBuilderActor<T>
where
    T: AddBridge + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        }

        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if button == &self.binding && !modifiers.alt() && modifiers.ctrl() {
                self.build_bridge().await;
            }
        }
        Capture::No
    }
}
