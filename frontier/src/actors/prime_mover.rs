use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use std::time::Duration;

use commons::async_std::task::sleep;
use commons::async_trait::async_trait;
use commons::bincode::{deserialize_from, serialize_into};
use commons::process::Step;
use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;
use commons::rand::seq::SliceRandom;
use commons::V2;
use isometric::Color;

use crate::avatar::{Avatar, AvatarLoad, AvatarTravelDuration, Journey};
use crate::bridge::Bridges;
use crate::nation::{NationColors, NationDescription};
use crate::resource::Resource;
use crate::route::{RouteKey, RoutesExt};
use crate::traits::{BuiltBridges, Micros, WithAvatars, WithRoutes, WithSettlements, WithWorld};
use crate::world::World;

pub struct PrimeMover<T> {
    cx: T,
    avatars: usize,
    travel_duration: Arc<AvatarTravelDuration>,
    durations: Durations,
    rng: SmallRng,
    active: HashMap<String, RouteKey>,
    colors: HashMap<String, NationColors>,
}

#[derive(Clone, Copy)]
struct Durations {
    pause_at_start: Duration,
    pause_in_middle: Duration,
    pause_at_end: Duration,
    pause_after_done: Duration,
    refresh_interval: Duration,
}

impl Default for Durations {
    fn default() -> Self {
        Durations {
            pause_at_start: Duration::from_secs(60 * 30),
            pause_in_middle: Duration::from_secs(60 * 60),
            pause_at_end: Duration::from_secs(60 * 30),
            pause_after_done: Duration::from_secs(60 * 60),
            refresh_interval: Duration::from_secs(1),
        }
    }
}

impl<T> PrimeMover<T>
where
    T: BuiltBridges + Micros + WithAvatars + WithRoutes + WithSettlements + WithWorld + Send + Sync,
{
    pub fn new(
        cx: T,
        avatars: usize,
        seed: u64,
        travel_duration: Arc<AvatarTravelDuration>,
        nation_descriptions: &[NationDescription],
    ) -> PrimeMover<T> {
        PrimeMover {
            cx,
            avatars,
            travel_duration,
            durations: Durations::default(),
            rng: SeedableRng::seed_from_u64(seed),
            active: HashMap::with_capacity(avatars),
            colors: Self::get_nation_colors(nation_descriptions),
        }
    }

    fn get_nation_colors(
        nation_descriptions: &[NationDescription],
    ) -> HashMap<String, NationColors> {
        nation_descriptions
            .iter()
            .map(|description| (description.name.clone(), description.colors))
            .collect()
    }

    pub async fn new_game(&self) {
        self.cx
            .mut_avatars(|avatars| {
                for i in 0..self.avatars {
                    avatars.all.insert(
                        i.to_string(),
                        Avatar {
                            name: i.to_string(),
                            journey: None,
                            color: Color::new(1.0, 0.0, 0.0, 1.0),
                            skin_color: Color::new(0.0, 0.0, 1.0, 1.0),
                        },
                    );
                }
            })
            .await;
    }

    async fn try_update_dormant(&mut self, dormant: HashSet<String>, micros: u128) {
        self.remove_from_active(&dormant);

        let keys = self.get_n_route_keys(dormant.len()).await;
        if keys.is_empty() {
            return;
        }

        let allocation = keys
            .into_iter()
            .zip(dormant.into_iter())
            .collect::<HashMap<_, _>>();
        let keys = allocation.keys().copied().collect::<Vec<_>>();
        let (journies, colors) = join!(self.get_journies(&keys, micros), self.get_colors(&keys));

        let avatars = self.get_avatars(allocation, journies, colors).await;
        self.update_avatars(avatars).await;
    }

    async fn get_dormant(&self, micros: &u128) -> HashSet<String> {
        self.cx
            .with_avatars(|avatars| {
                avatars
                    .all
                    .values()
                    .filter(|avatar| Some(&avatar.name) != avatars.selected.as_ref())
                    .filter(|avatar| {
                        is_dormant(avatar, micros, &self.durations.pause_after_done.as_micros())
                    })
                    .map(|avatar| avatar.name.clone())
                    .collect()
            })
            .await
    }

    fn remove_from_active(&mut self, dormant: &HashSet<String>) {
        self.active.retain(|key, _| !dormant.contains(key));
    }

    async fn get_n_route_keys(&mut self, n: usize) -> Vec<RouteKey> {
        let candidates = self.get_candidates().await;

        let selected_keys =
            candidates.choose_multiple_weighted(&mut self.rng, n, |candidate| candidate.1 as f64);
        return selected_keys
            .unwrap()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
    }

    async fn get_candidates(&self) -> Vec<(RouteKey, u128)> {
        let active_keys = self.active.values().cloned().collect::<HashSet<_>>();
        self.cx
            .with_routes(|routes| {
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

    async fn get_journies(&self, keys: &[RouteKey], start_at: u128) -> HashMap<RouteKey, Journey> {
        let (paths, bridges) = join!(self.get_paths(&keys), self.cx.built_bridges());
        self.get_journies_from_paths(paths, start_at, &bridges)
            .await
    }

    async fn get_paths(&self, keys: &[RouteKey]) -> HashMap<RouteKey, Vec<V2<usize>>> {
        self.cx
            .with_routes(|routes| {
                keys.iter()
                    .flat_map(|key| {
                        routes
                            .get_route(key)
                            .map(|route| (*key, route.path.clone()))
                    })
                    .collect()
            })
            .await
    }

    async fn get_journies_from_paths(
        &self,
        paths: HashMap<RouteKey, Vec<V2<usize>>>,
        start_at: u128,
        bridges: &Bridges,
    ) -> HashMap<RouteKey, Journey> {
        self.cx
            .with_world(|world| {
                paths
                    .into_iter()
                    .map(|(key, outbound)| {
                        let journey = Self::get_out_and_back_journey(
                            world,
                            &self.travel_duration,
                            &self.durations,
                            &start_at,
                            outbound,
                            key.resource,
                            bridges,
                        );
                        (key, journey)
                    })
                    .collect()
            })
            .await
    }

    fn get_out_and_back_journey(
        world: &World,
        travel_duration: &AvatarTravelDuration,
        durations: &Durations,
        start_at: &u128,
        outbound: Vec<V2<usize>>,
        resource: Resource,
        bridges: &Bridges,
    ) -> Journey {
        let mut inbound = outbound.clone();
        inbound.reverse();

        let outbound = Journey::new(
            world,
            outbound,
            travel_duration,
            travel_duration.travel_mode_fn(),
            *start_at,
            bridges,
        )
        .with_pause_at_start(durations.pause_at_start.as_micros())
        .with_pause_at_end(durations.pause_in_middle.as_micros() / 2);

        let inbound_start = outbound.final_frame().arrival;
        let inbound = Journey::new(
            world,
            inbound,
            travel_duration,
            travel_duration.travel_mode_fn(),
            inbound_start,
            bridges,
        )
        .with_pause_at_start(durations.pause_in_middle.as_micros() / 2)
        .with_pause_at_end(durations.pause_at_end.as_micros())
        .with_load(AvatarLoad::Resource(resource));

        outbound.append(inbound).unwrap()
    }

    async fn get_colors(&self, keys: &[RouteKey]) -> HashMap<RouteKey, NationColors> {
        self.get_nations(&keys)
            .await
            .into_iter()
            .flat_map(|(key, nation)| self.colors.get(&nation).map(|colors| (key, *colors)))
            .collect()
    }

    async fn get_nations(&self, keys: &[RouteKey]) -> HashMap<RouteKey, String> {
        self.cx
            .with_settlements(|settlements| {
                keys.iter()
                    .flat_map(|key| {
                        settlements
                            .get(&key.settlement)
                            .map(|settlement| (*key, settlement.nation.clone()))
                    })
                    .collect()
            })
            .await
    }

    async fn get_avatars(
        &mut self,
        allocation: HashMap<RouteKey, String>,
        mut journies: HashMap<RouteKey, Journey>,
        colors: HashMap<RouteKey, NationColors>,
    ) -> HashMap<String, Avatar> {
        let mut out = HashMap::new();
        for (key, avatar) in allocation {
            let path = unwrap_or!(journies.remove(&key), continue);
            let colors = unwrap_or!(colors.get(&key), continue);
            self.active.insert(avatar.clone(), key);
            out.insert(
                avatar.clone(),
                Avatar {
                    name: avatar,
                    journey: Some(path),
                    color: colors.primary,
                    skin_color: colors.skin,
                },
            );
        }
        out
    }

    async fn update_avatars(&self, updated: HashMap<String, Avatar>) {
        if updated.is_empty() {
            return;
        }
        self.cx
            .mut_avatars(|avatars| {
                for (name, avatar) in updated {
                    avatars.all.insert(name, avatar);
                }
            })
            .await
    }

    pub fn save(&self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        serialize_into(&mut file, &self.active).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.active = deserialize_from(file).unwrap();
    }

    fn get_path(path: &str) -> String {
        format!("{}.prime_mover", path)
    }
}

fn is_dormant(avatar: &Avatar, at: &u128, pause_after_done_micros: &u128) -> bool {
    match &avatar.journey {
        Some(journey) => journey.done(&(at - pause_after_done_micros)),
        None => true,
    }
}

#[async_trait]
impl<T> Step for PrimeMover<T>
where
    T: BuiltBridges + Micros + WithAvatars + WithRoutes + WithSettlements + WithWorld + Send + Sync,
{
    async fn step(&mut self) {
        let micros = self.cx.micros().await;
        let dormant = self.get_dormant(&micros).await;

        if !dormant.is_empty() {
            self.try_update_dormant(dormant, micros).await;
        }

        sleep(self.durations.refresh_interval).await;
    }
}
