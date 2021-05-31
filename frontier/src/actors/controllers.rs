use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use commons::async_trait::async_trait;
use commons::grid::get_corners;
use commons::grid::Grid;
use commons::index2d::Vec2D;
use commons::v2;
use commons::M;
use commons::V2;
use isometric::Button;
use isometric::ElementState;
use isometric::Event;
use isometric::VirtualKeyCode;

use crate::settlement::SettlementClass::Town;
use crate::system::Capture;
use crate::system::HandleEngineEvent;
use crate::traits::{PathfinderForRoutes, Settlements, WithControllers, WithPathfinder};

pub struct ControllersActor<T> {
    cx: T,
}

impl<T> ControllersActor<T>
where
    T: PathfinderForRoutes + Settlements + WithControllers,
{
    pub fn new(t: T) -> ControllersActor<T> {
        ControllersActor { cx: t }
    }

    pub async fn recompute(&self) {
        let origin_to_positions = self
            .cx
            .settlements()
            .await
            .into_iter()
            .filter(|settlement| settlement.class == Town)
            .map(|settlement| (settlement.position, get_corners(&settlement.position)))
            .collect::<HashMap<_, _>>();

        let closest_origins = self
            .cx
            .routes_pathfinder()
            .with_pathfinder(|pathfinder| pathfinder.closest_origins(&origin_to_positions))
            .await;

        let new_controllers =
            M::from_fn(closest_origins.width(), closest_origins.height(), |x, y| {
                get_controller(&closest_origins, &v2(x, y))
            });

        self.cx
            .mut_controllers(|controllers| *controllers = new_controllers)
            .await;
    }
}

fn get_controller(
    closest_origins: &Vec2D<HashSet<V2<usize>>>,
    position: &V2<usize>,
) -> Option<V2<usize>> {
    let mut candidates: HashMap<V2<usize>, usize> = hashmap! {};
    for controller in get_corners(position)
        .iter()
        .flat_map(|position| closest_origins.get_cell(position))
        .flatten()
    {
        *candidates.entry(*controller).or_default() += 1;
    }
    candidates
        .into_iter()
        .max_by(|a, b| {
            a.1.cmp(&b.1)
                .then(a.0.x.cmp(&b.0.x))
                .then(a.0.y.cmp(&b.0.y))
        })
        .map(|(a, _)| a)
}

#[async_trait]
impl<T> HandleEngineEvent for ControllersActor<T>
where
    T: PathfinderForRoutes + Settlements + WithControllers + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if button == &Button::Key(VirtualKeyCode::C) && !modifiers.alt() && modifiers.ctrl() {
                self.recompute().await;
            }
        }
        Capture::No
    }
}
