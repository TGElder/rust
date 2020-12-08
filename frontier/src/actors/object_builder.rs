use crate::reactor::ActorTraits;
use crate::traits::{RemoveWorldObject, SetWorldObject};
use crate::world::WorldObject;
use commons::async_channel::{Receiver, RecvError};
use commons::async_trait::async_trait;
use commons::fn_sender::{FnMessageExt, FnReceiver};
use commons::futures::future::FutureExt;
use commons::rand::rngs::SmallRng;
use commons::rand::{Rng, SeedableRng};
use commons::V2;
use isometric::coords::WorldCoord;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;

pub struct ObjectBuilder<T> {
    x: T,
    rx: FnReceiver<ObjectBuilder<T>>,
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

#[async_trait]
impl<T> ActorTraits for ObjectBuilder<T>
where
    T: RemoveWorldObject + SetWorldObject + Send + Sync + 'static,
{
    async fn run(mut self) -> Self {
        while self.run {
            self.step().await;
        }
        self
    }

    fn resume(&mut self) {
        self.run = true;
    }

    fn shutdown(&mut self) {
        self.run = false;
    }
}

impl<T> ObjectBuilder<T>
where
    T: RemoveWorldObject + SetWorldObject + Send,
{
    pub fn new(
        x: T,
        rx: FnReceiver<ObjectBuilder<T>>,
        engine_rx: Receiver<Arc<Event>>,
        seed: u64,
    ) -> ObjectBuilder<T> {
        ObjectBuilder {
            x,
            rx,
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

    async fn step(&mut self) {
        select! {
            mut message = self.rx.get_message().fuse() => message.apply(self).await,
            event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
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
            modifiers:
                ModifiersState {
                    alt: false,
                    ctrl: true,
                    ..
                },
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
