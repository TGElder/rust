use crate::avatar::{AvatarState, Rotation, Vehicle};
use crate::system::HandleEngineEvent;
use crate::traits::{RevealAll, SendAvatars, SendWorld, UpdateAvatar, Visibility};
use commons::async_trait::async_trait;
use commons::grid::Grid;
use isometric::{coords::*, Event};
use isometric::{Button, ElementState, VirtualKeyCode};
use std::default::Default;

pub struct Cheats<T> {
    tx: T,
    bindings: CheatBindings,
    world_coord: Option<WorldCoord>,
}

pub struct CheatBindings {
    reveal_all: Button,
    move_avatar: Button,
    remove_avatar: Button,
}

impl Default for CheatBindings {
    fn default() -> CheatBindings {
        CheatBindings {
            reveal_all: Button::Key(VirtualKeyCode::V),
            move_avatar: Button::Key(VirtualKeyCode::H),
            remove_avatar: Button::Key(VirtualKeyCode::R),
        }
    }
}

impl<T> Cheats<T>
where
    T: RevealAll
        + SendAvatars
        + SendWorld
        + UpdateAvatar
        + Visibility
        + Clone
        + Send
        + Sync
        + 'static,
{
    pub fn new(tx: T) -> Cheats<T> {
        Cheats {
            tx,
            bindings: CheatBindings::default(),
            world_coord: None,
        }
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    async fn reveal_all(&mut self) {
        self.tx.reveal_all().await;
        self.tx.disable_visibility_computation();
    }

    async fn move_avatar(&mut self) {
        let world_coord = unwrap_or!(self.world_coord, return);
        let position = world_coord.to_v2_round();

        let name = unwrap_or!(self.selected_avatar_name().await, return);

        let moved = self
            .tx
            .send_world(move |world| AvatarState::Stationary {
                elevation: world
                    .get_cell_unsafe(&position)
                    .elevation
                    .max(world.sea_level()),
                position,
                rotation: Rotation::Down,
                vehicle: Vehicle::None,
            })
            .await;

        self.tx.update_avatar_state(name, moved).await;
    }

    async fn selected_avatar_name(&self) -> Option<String> {
        self.tx
            .send_avatars(|avatars| avatars.selected.clone())
            .await
    }

    async fn remove_avatar(&mut self) {
        if let Some(name) = self.selected_avatar_name().await {
            self.tx.update_avatar_state(name, AvatarState::Absent).await;
        }
    }
}

#[async_trait]
impl<T> HandleEngineEvent for Cheats<T>
where
    T: RevealAll
        + SendAvatars
        + SendWorld
        + UpdateAvatar
        + Visibility
        + Clone
        + Send
        + Sync
        + 'static,
{
    async fn handle_engine_event(&mut self, event: std::sync::Arc<isometric::Event>) {
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
            if button == &self.bindings.reveal_all && modifiers.alt() {
                self.reveal_all().await;
            } else if button == &self.bindings.move_avatar && modifiers.alt() {
                self.move_avatar().await;
            } else if button == &self.bindings.remove_avatar && modifiers.alt() {
                self.remove_avatar().await;
            }
        }
    }
}
