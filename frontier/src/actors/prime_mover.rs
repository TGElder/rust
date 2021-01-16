use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use commons::async_std::task::sleep;
use commons::async_trait::async_trait;
use commons::process::Step;
use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;
use commons::rand::seq::SliceRandom;
use commons::V2;
use isometric::Color;

use crate::avatar::{Avatar, AvatarLoad, AvatarState, AvatarTravelDuration, TravelArgs};
use crate::route::{RouteKey, RoutesExt};
use crate::traits::{Micros, SendAvatars, SendRoutes, SendWorld};

pub struct PrimeMover<T> {
    tx: T,
    avatars: usize,
    travel_duration: Arc<AvatarTravelDuration>,
    sleep: Duration,
    rng: SmallRng,
}

impl<T> PrimeMover<T>
where
    T: Micros + SendAvatars + SendRoutes + SendWorld,
{
    pub fn new(
        tx: T,
        avatars: usize,
        seed: u64,
        travel_duration: Arc<AvatarTravelDuration>,
    ) -> PrimeMover<T> {
        PrimeMover {
            tx,
            avatars,
            travel_duration,
            sleep: Duration::from_secs(1),
            rng: SeedableRng::seed_from_u64(seed),
        }
    }

    pub async fn new_game(&self) {
        let count = self.avatars;
        self.tx
            .send_avatars(move |avatars| {
                for i in 0..count {
                    avatars.all.insert(
                        i.to_string(),
                        Avatar {
                            name: i.to_string(),
                            state: AvatarState::Absent,
                            load: AvatarLoad::None,
                            color: Color::new(1.0, 0.0, 0.0, 1.0),
                            skin_color: Color::new(0.0, 0.0, 1.0, 1.0),
                        },
                    );
                }
            })
            .await;
    }

    async fn get_dormant(&self, micros: u128) -> HashSet<String> {
        self.tx
            .send_avatars(move |avatars| {
                avatars
                    .all
                    .values()
                    .filter(|avatar| Some(&avatar.name) != avatars.selected.as_ref())
                    .filter(|avatar| is_done(avatar, &micros))
                    .map(|avatar| avatar.name.clone())
                    .collect()
            })
            .await
    }

    async fn get_n_avatar_states(&mut self, n: usize, micros: &u128) -> Vec<AvatarState> {
        let candidates = self.get_candidates().await;

        let selected_keys =
            candidates.choose_multiple_weighted(&mut self.rng, n, |candidate| candidate.1 as f64);
        let selected_keys = ok_or!(selected_keys, return vec![])
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();

        let routes = self.get_paths(selected_keys).await;

        return self.get_avatar_states(routes, *micros).await;
    }

    async fn get_candidates(&self) -> Vec<(RouteKey, u128)> {
        self.tx
            .send_routes(|routes| {
                routes
                    .values()
                    .flat_map(|route_set| route_set.iter())
                    .filter(|(_, route)| route.path.len() > 1)
                    .map(|(key, route)| (*key, route.duration.as_micros()))
                    .collect()
            })
            .await
    }

    async fn get_paths(&self, keys: Vec<RouteKey>) -> Vec<Vec<V2<usize>>> {
        self.tx
            .send_routes(move |routes| {
                keys.iter()
                    .flat_map(|key| routes.get_route(key))
                    .map(|route| route.path.clone())
                    .collect()
            })
            .await
    }

    async fn get_avatar_states(
        &self,
        paths: Vec<Vec<V2<usize>>>,
        start_at: u128,
    ) -> Vec<AvatarState> {
        let travel_duration = self.travel_duration.clone();
        self.tx
            .send_world(move |world| {
                paths
                    .into_iter()
                    .flat_map(|path| {
                        AvatarState::Absent.travel(TravelArgs {
                            world,
                            positions: path,
                            travel_duration: travel_duration.as_ref(),
                            vehicle_fn: travel_duration.travel_mode_fn(),
                            start_at,
                            pause_at_start: None,
                            pause_at_end: None,
                        })
                    })
                    .collect()
            })
            .await
    }

    async fn update_avatars(&self, allocation: HashMap<String, AvatarState>) {
        self.tx
            .send_avatars(move |avatars| {
                for (name, state) in allocation {
                    if let Some(avatar) = avatars.all.get_mut(&name) {
                        avatar.state = state;
                    }
                }
            })
            .await
    }
}

fn is_done(avatar: &Avatar, at: &u128) -> bool {
    match &avatar.state {
        AvatarState::Stationary { .. } => true,
        AvatarState::Walking(path) => path.done(at),
        AvatarState::Absent => true,
    }
}

#[async_trait]
impl<T> Step for PrimeMover<T>
where
    T: Micros + SendAvatars + SendRoutes + SendWorld + Send + Sync,
{
    async fn step(&mut self) {
        let micros = self.tx.micros().await;
        let dormant = self.get_dormant(micros).await;

        if (dormant.is_empty()) {
            return;
        }

        let avatar_states = self.get_n_avatar_states(dormant.len(), &micros).await;

        let allocation = dormant.into_iter().zip(avatar_states.into_iter()).collect();
        self.update_avatars(allocation).await;

        sleep(self.sleep).await;
    }
}
