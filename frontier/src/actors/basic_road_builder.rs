use crate::avatar::{Avatar, AvatarTravelDuration, Journey};
use crate::road_builder::{AutoRoadTravelDuration, RoadBuildMode, RoadBuilderResult};
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{Micros, SelectedAvatar, SendWorld, UpdateAvatarJourney, UpdateRoads};
use crate::travel_duration::TravelDuration;
use commons::async_trait::async_trait;
use commons::edge::Edge;
use commons::{unwrap_or, V2};
use isometric::{Button, ElementState, Event, VirtualKeyCode};
use std::sync::Arc;

pub struct BasicRoadBuilder<T> {
    tx: T,
    avatar_travel_duration: Arc<AvatarTravelDuration>,
    road_build_travel_duration: Arc<AutoRoadTravelDuration>,
    binding: Button,
}

impl<T> BasicRoadBuilder<T>
where
    T: Micros + SelectedAvatar + SendWorld + UpdateAvatarJourney + UpdateRoads,
{
    pub fn new(
        tx: T,
        avatar_travel_duration: Arc<AvatarTravelDuration>,
        road_build_travel_duration: Arc<AutoRoadTravelDuration>,
    ) -> BasicRoadBuilder<T> {
        BasicRoadBuilder {
            tx,
            avatar_travel_duration,
            road_build_travel_duration,
            binding: Button::Key(VirtualKeyCode::R),
        }
    }

    async fn build_road(&mut self) {
        let (micros, selected_avatar) = join!(self.tx.micros(), self.tx.selected_avatar());
        let selected_avatar = unwrap_or!(selected_avatar, return);
        let forward_path = unwrap_or!(self.get_forward_path(&selected_avatar, &micros), return);
        if !self.is_buildable(forward_path.clone()).await {
            return;
        }

        self.move_avatar(selected_avatar.name, forward_path.clone(), micros)
            .await;
        self.update_roads(forward_path).await;
    }

    fn get_forward_path(&self, avatar: &Avatar, micros: &u128) -> Option<Vec<V2<usize>>> {
        match avatar.journey.as_ref() {
            Some(journey) => {
                if journey.done(micros) {
                    Some(journey.forward_path())
                } else {
                    None
                }
            }
            None => None,
        }
    }

    async fn is_buildable(&self, forward_path: Vec<V2<usize>>) -> bool {
        let travel_duration = self.road_build_travel_duration.clone();
        self.tx
            .send_world(move |world| {
                travel_duration
                    .get_duration(world, &forward_path[0], &forward_path[1])
                    .is_some()
            })
            .await
    }

    async fn move_avatar(&self, name: String, forward_path: Vec<V2<usize>>, micros: u128) {
        let journey = self.get_journey(forward_path.clone(), micros).await;
        self.tx.update_avatar_journey(name, Some(journey)).await;
    }

    async fn get_journey(&self, forward_path: Vec<V2<usize>>, start_at: u128) -> Journey {
        let travel_duration = self.avatar_travel_duration.clone();
        self.tx
            .send_world(move |world| {
                Journey::new(
                    world,
                    forward_path,
                    travel_duration.as_ref(),
                    travel_duration.travel_mode_fn(),
                    start_at,
                )
            })
            .await
    }

    async fn update_roads(&self, forward_path: Vec<V2<usize>>) {
        let mode = self.get_mode(forward_path.clone()).await;
        let result = RoadBuilderResult::new(vec![forward_path[0], forward_path[1]], mode);
        self.tx.update_roads(result).await;
    }

    async fn get_mode(&self, forward_path: Vec<V2<usize>>) -> RoadBuildMode {
        self.tx
            .send_world(move |world| {
                let edge = Edge::new(forward_path[0], forward_path[1]);
                if world.is_road(&edge) {
                    RoadBuildMode::Demolish
                } else {
                    RoadBuildMode::Build
                }
            })
            .await
    }
}

#[async_trait]
impl<T> HandleEngineEvent for BasicRoadBuilder<T>
where
    T: Micros
        + SelectedAvatar
        + SendWorld
        + UpdateAvatarJourney
        + UpdateRoads
        + Send
        + Sync
        + 'static,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if *button == self.binding && !modifiers.alt() && modifiers.ctrl() {
                self.build_road().await;
            }
        }
        Capture::No
    }
}
