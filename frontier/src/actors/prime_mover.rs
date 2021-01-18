use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};
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

use crate::avatar::{Avatar, AvatarLoad, AvatarTravelDuration, Path};
use crate::route::{RouteKey, RoutesExt};
use crate::traits::{Micros, SendAvatars, SendRoutes, SendWorld};

pub struct PrimeMover<T> {
    tx: T,
    avatars: usize,
    travel_duration: Arc<AvatarTravelDuration>,
    sleep: Duration,
    rng: SmallRng,
    active: HashMap<String, RouteKey>,
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
            active: HashMap::with_capacity(avatars),
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
                            path: None,
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
                    .filter(|avatar| is_dormant(avatar, &micros))
                    .map(|avatar| avatar.name.clone())
                    .collect()
            })
            .await
    }

    fn remove_from_active(&mut self, dormant: &HashSet<String>) {
        self.active.retain(|key, _| !dormant.contains(key));
    }

    async fn get_n_avatar_paths(&mut self, n: usize, micros: &u128) -> HashMap<RouteKey, Path> {
        let candidates = self.get_candidates().await;

        let selected_keys =
            candidates.choose_multiple_weighted(&mut self.rng, n, |candidate| candidate.1 as f64);
        let selected_keys = selected_keys
            .unwrap()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();

        let routes = self.get_paths(selected_keys).await;

        return self.get_avatar_paths(routes, *micros).await;
    }

    async fn get_candidates(&self) -> Vec<(RouteKey, u128)> {
        let active_keys = self.active.values().cloned().collect::<HashSet<_>>();
        self.tx
            .send_routes(move |routes| {
                routes
                    .values()
                    .flat_map(|route_set| route_set.iter())
                    .filter(|(_, route)| route.path.len() > 1)
                    .filter(|(key, _)| !active_keys.contains(key))
                    .map(|(key, route)| (*key, route.duration.as_micros()))
                    .collect()
            })
            .await
    }

    async fn get_paths(&self, keys: Vec<RouteKey>) -> HashMap<RouteKey, Vec<V2<usize>>> {
        self.tx
            .send_routes(move |routes| {
                keys.into_iter()
                    .flat_map(|key| {
                        routes
                            .get_route(&key)
                            .map(|route| (key, route.path.clone()))
                    })
                    .collect()
            })
            .await
    }

    async fn get_avatar_paths(
        &self,
        paths: HashMap<RouteKey, Vec<V2<usize>>>,
        start_at: u128,
    ) -> HashMap<RouteKey, Path> {
        let travel_duration = self.travel_duration.clone();
        self.tx
            .send_world(move |world| {
                paths
                    .into_iter()
                    .map(|(key, path)| {
                        let path = Path::new(
                            world,
                            path,
                            travel_duration.as_ref(),
                            travel_duration.travel_mode_fn(),
                            start_at,
                        );
                        (key, path)
                    })
                    .collect()
            })
            .await
    }

    fn add_to_active(&mut self, allocation: &HashMap<String, (RouteKey, Path)>) {
        for (avatar, (key, _)) in allocation {
            self.active.insert(avatar.clone(), *key);
        }
    }

    async fn update_avatars(&self, allocation: HashMap<String, (RouteKey, Path)>) {
        self.tx
            .send_avatars(move |avatars| {
                for (name, (_, path)) in allocation {
                    if let Some(avatar) = avatars.all.get_mut(&name) {
                        avatar.path = Some(path);
                    }
                }
            })
            .await
    }

    pub fn save(&self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.active).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.active = bincode::deserialize_from(file).unwrap();
    }

    fn get_path(path: &str) -> String {
        format!("{}.prime_mover", path)
    }
}

fn is_dormant(avatar: &Avatar, at: &u128) -> bool {
    match &avatar.path {
        Some(path) => path.done(at),
        None => true,
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

        if (!dormant.is_empty()) {
            self.remove_from_active(&dormant);

            let avatar_paths = self.get_n_avatar_paths(dormant.len(), &micros).await;

            let allocation = dormant.into_iter().zip(avatar_paths.into_iter()).collect();
            self.add_to_active(&allocation);
            self.update_avatars(allocation).await;
        }

        sleep(self.sleep).await;
    }
}
