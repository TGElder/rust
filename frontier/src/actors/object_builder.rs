use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{RemoveWorldObjects, SetWorldObjects};
use crate::world::WorldObject;
use commons::async_trait::async_trait;
use commons::rand::rngs::SmallRng;
use commons::rand::{Rng, SeedableRng};
use commons::V2;
use isometric::coords::WorldCoord;
use isometric::{Button, ElementState, Event, VirtualKeyCode};
use std::sync::Arc;

pub struct ObjectBuilderActor<T> {
    cx: T,
    rng: SmallRng,
    bindings: ObjectBuilderBindings,
    world_coord: Option<WorldCoord>,
}

struct ObjectBuilderBindings {
    build_crop: Button,
    demolish: Button,
}

impl<T> ObjectBuilderActor<T>
where
    T: RemoveWorldObjects + SetWorldObjects,
{
    pub fn new(cx: T, seed: u64) -> ObjectBuilderActor<T> {
        ObjectBuilderActor {
            cx,
            rng: SeedableRng::seed_from_u64(seed),
            bindings: ObjectBuilderBindings {
                build_crop: Button::Key(VirtualKeyCode::F),
                demolish: Button::Key(VirtualKeyCode::U),
            },
            world_coord: None,
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
            self.cx
                .set_world_objects(&hashmap! {position => object})
                .await;
        }
    }

    async fn clear_object_at_cursor(&self) {
        if let Some(position) = self.get_position() {
            self.cx.remove_world_objects(&hashset! {position}).await;
        }
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|world_coord| world_coord.to_v2_floor())
    }
}

#[async_trait]
impl<T> HandleEngineEvent for ObjectBuilderActor<T>
where
    T: RemoveWorldObjects + SetWorldObjects + Send + Sync + 'static,
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
            if button == &self.bindings.build_crop && !modifiers.alt() && modifiers.ctrl() {
                self.build_farm_at_cursor().await;
            } else if button == &self.bindings.demolish && !modifiers.alt() && modifiers.ctrl() {
                self.clear_object_at_cursor().await;
            }
        }
        Capture::No
    }
}
