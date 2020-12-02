use crate::traits::{RemoveWorldObject, SetWorldObject};
use crate::world::WorldObject;
use commons::async_channel::{Receiver, RecvError};
use commons::futures::future::FutureExt;
use commons::rand::rngs::SmallRng;
use commons::rand::{Rng, SeedableRng};
use commons::V2;
use isometric::coords::WorldCoord;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

pub struct ObjectBuilder<T> {
    x: T,
    engine_rx: Receiver<Arc<Event>>,
    rng: SmallRng,
    bindings: ObjectBuilderBindings,
    world_coord: Option<WorldCoord>,
    run: bool,
}

struct ObjectBuilderBindings {
    build_crop: Button,
    demolish: Button,
}

impl<T> ObjectBuilder<T>
where
    T: RemoveWorldObject + SetWorldObject,
{
    pub fn new(x: T, engine_rx: Receiver<Arc<Event>>, seed: u64) -> ObjectBuilder<T> {
        ObjectBuilder {
            x,
            engine_rx,
            rng: SeedableRng::seed_from_u64(seed),
            bindings: ObjectBuilderBindings {
                build_crop: Button::Key(VirtualKeyCode::F),
                demolish: Button::Key(VirtualKeyCode::U),
            },
            world_coord: None,
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        let event: Arc<Event> = event.unwrap();

        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        }

        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.bindings.build_crop {
                self.build_farm_at_cursor().await;
            } else if button == &self.bindings.demolish {
                self.clear_object_at_cursor().await;
            }
        }

        if let Event::Shutdown = *event {
            self.shutdown();
        }
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    async fn build_farm_at_cursor(&mut self) {
        let rotated = self.rng.gen();
        self.build_object_at_cursor(WorldObject::Crop { rotated })
            .await;
    }

    async fn build_object_at_cursor(&self, object: WorldObject) {
        if let Some(position) = self.get_position() {
            self.x.set_world_object(object, position).await;
        }
    }

    async fn clear_object_at_cursor(&self) {
        if let Some(position) = self.get_position() {
            self.x.remove_world_object(position).await;
        }
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|world_coord| world_coord.to_v2_floor())
    }

    fn shutdown(&mut self) {
        self.run = false;
    }
}
