use std::{sync::Arc, time::Duration};

use commons::{
    async_std::task::sleep, async_trait::async_trait, grid::Grid, process::Step, v2, V2,
};
use isometric::{Button, ElementState, Event, VirtualKeyCode};

use crate::{
    avatar::{Avatar, Frame, Rotation},
    system::{Capture, HandleEngineEvent},
    traits::{has::HasParameters, Micros, SelectedAvatar, WithWorld},
};

pub struct RiverExplorer<T> {
    cx: T,
    active: bool,
    parameters: RiverExplorerParameters,
}

pub struct RiverExplorerParameters {
    pub refresh_interval: Duration,
    pub binding: Button,
    pub min_navigable_river_width: f32,
}

impl Default for RiverExplorerParameters {
    fn default() -> RiverExplorerParameters {
        RiverExplorerParameters {
            refresh_interval: Duration::from_millis(100),
            binding: Button::Key(VirtualKeyCode::X),
            min_navigable_river_width: 0.1,
        }
    }
}

trait RiverExplorerTraits: HasParameters + Micros + SelectedAvatar + WithWorld {}

impl<T> RiverExplorer<T>
where
    T: RiverExplorerTraits,
{
    pub fn new(cx: T, parameters: RiverExplorerParameters) -> RiverExplorer<T> {
        RiverExplorer {
            cx,
            active: false,
            parameters,
        }
    }

    async fn explore(&self) {
        let micros = self.cx.micros().await;
        let journey = match self.cx.selected_avatar().await {
            Some(Avatar {
                journey: Some(journey),
                ..
            }) => journey,
            _ => return,
        };

        if !journey.done(&micros) {
            return;
        }

        let Frame {
            position, rotation, ..
        } = journey.final_frame();
        let grid_width = self.cx.parameters().width;
        let behind = unwrap_or!(behind(&position, &rotation, &grid_width), return);
        let forward_candidates = forward_candidates(&position, &rotation, &grid_width);
        if forward_candidates.is_empty() {
            return;
        }

        if forward_candidates.is_empty() {
            return;
        }
    }

    async fn choose_candidate(
        &self,
        behind: &V2<usize>,
        position: &V2<usize>,
        forward_candidates: Vec<Rotation>,
    ) -> Option<Rotation> {
        let min_navigable_river_width = self.parameters.min_navigable_river_width;
        self.cx
            .with_world(|world| {
                let behind = unwrap_or!(world.get_cell(behind), return None);
                if behind.river.width() < min_navigable_river_width {
                    return None;
                }
                let position = unwrap_or!(world.get_cell(position), return None);
                if position.river.width() < min_navigable_river_width {
                    return None;
                }
                forward_candidates
                    .into_iter()
                    .flat_map(|candidate| {
                        world
                            .offset(&position.position, offset(&candidate))
                            .and_then(|position| world.get_cell(&position))
                            .map(|cell| (candidate, cell))
                    })
                    .filter(|(_, forward)| forward.river.width() < min_navigable_river_width)
                    .filter(|(_, forward)| {
                        (behind.elevation < position.elevation)
                            == (position.elevation < forward.elevation)
                    })
                    .max_by(|a, b| {
                        unsafe_ordering(a.1.river.width, b.1.river.width)
                    })
                    .map(|(candidate, _)| candidate)
            })
            .await
    }
}

fn forward_candidates(
    position: &V2<usize>,
    rotation: &Rotation,
    grid_width: &usize,
) -> Vec<Rotation> {
    match rotation {
        Rotation::Left => vec![Rotation::Down, Rotation::Left, Rotation::Up],
        Rotation::Up => vec![Rotation::Left, Rotation::Up, Rotation::Right],
        Rotation::Right => vec![Rotation::Up, Rotation::Right, Rotation::Down],
        Rotation::Down => vec![Rotation::Right, Rotation::Down, Rotation::Left],
    }
}

fn offset(rotation: &Rotation) -> V2<i32> {
    match rotation {
        Rotation::Left => v2(-1, 0),
        Rotation::Up => v2(0, 1),
        Rotation::Right => v2(1, 0),
        Rotation::Down => v2(0, -1),
    }
}

fn behind(position: &V2<usize>, rotation: &Rotation, grid_width: &usize) -> Option<V2<usize>> {
    let behind = v2(position.x as i32, position.y as i32) + offset(&rotation) * -1;
    if behind.x >= 0 && behind.y >= 0 && behind.x as usize < *grid_width && behind.y as usize < *grid_width {
        Some(v2(behind.x as usize, behind.y as usize))
    } else {
        None
    }
}

#[async_trait]
impl<T> Step for RiverExplorer<T>
where
    T: Send + Sync,
{
    async fn step(&mut self) {
        if self.active {
            self.explore().await;
        }

        sleep(self.parameters.refresh_interval).await;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for RiverExplorer<T>
where
    T: Send + Sync + 'static,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if *button == self.parameters.binding && !modifiers.alt() && modifiers.ctrl() {
                self.active = !self.active;
            }
        }
        Capture::No
    }
}
