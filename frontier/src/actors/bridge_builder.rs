use crate::avatar::Vehicle;
use crate::bridges::BridgeType::Built;
use crate::bridges::{Bridge, Pier};
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{AddBridge, RemoveBridge, WithWorld};
use commons::async_trait::async_trait;
use commons::edge::Edge;
use commons::grid::Grid;
use commons::V2;
use isometric::coords::WorldCoord;
use isometric::{Button, ElementState, Event, VirtualKeyCode};
use std::sync::Arc;

pub struct BridgeBuilderActor<T> {
    cx: T,
    binding: Button,
    from: Option<V2<usize>>,
    world_coord: Option<WorldCoord>,
    parameters: BridgeBuilderParameters,
}

pub struct BridgeBuilderParameters {
    pub min_length: usize,
    pub max_length: usize,
    pub max_gradient: f32,
}

impl Default for BridgeBuilderParameters {
    fn default() -> Self {
        BridgeBuilderParameters {
            min_length: 2,
            max_length: 3,
            max_gradient: 0.5,
        }
    }
}

impl<T> BridgeBuilderActor<T>
where
    T: AddBridge + RemoveBridge + WithWorld,
{
    pub fn new(cx: T, parameters: BridgeBuilderParameters) -> BridgeBuilderActor<T> {
        BridgeBuilderActor {
            cx,
            binding: Button::Key(VirtualKeyCode::G),
            from: None,
            world_coord: None,
            parameters,
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
        let edge = ok_or!(Edge::new_safe(from, to), return);

        if !self.is_valid_bridge(&edge).await {
            return;
        }

        let (from_z, to_z) = match self
            .cx
            .with_world(|world| {
                (
                    world.get_cell(edge.from()).map(|cell| cell.elevation),
                    world.get_cell(edge.to()).map(|cell| cell.elevation),
                )
            })
            .await
        {
            (Some(from_z), Some(to_z)) => (from_z, to_z),
            _ => return,
        };

        let bridge = Bridge {
            piers: vec![
                Pier {
                    position: *edge.from(),
                    elevation: from_z,
                    platform: true,
                },
                Pier {
                    position: *edge.to(),
                    elevation: to_z,
                    platform: true,
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: Built,
        };
        if let Ok(bridge) = bridge.validate() {
            if self.cx.remove_bridge(bridge.clone()).await {
                return;
            } else {
                self.cx.add_bridge(bridge).await;
            }
        }
    }

    async fn is_valid_bridge(&self, edge: &Edge) -> bool {
        let length = edge.length();
        if length < self.parameters.min_length || length > self.parameters.max_length {
            return false;
        }
        let rise = unwrap_or!(self.get_rise(edge).await, return false);
        (rise / length as f32) <= self.parameters.max_gradient
    }

    async fn get_rise(&self, edge: &Edge) -> Option<f32> {
        self.cx
            .with_world(
                |world| match (world.get_cell(edge.from()), world.get_cell(edge.to())) {
                    (Some(from), Some(to)) => Some((from.elevation - to.elevation).abs()),
                    _ => None,
                },
            )
            .await
    }
}

#[async_trait]
impl<T> HandleEngineEvent for BridgeBuilderActor<T>
where
    T: AddBridge + RemoveBridge + WithWorld + Send + Sync,
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
