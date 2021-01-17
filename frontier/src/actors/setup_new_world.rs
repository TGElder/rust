use crate::avatar::{Avatar, AvatarLoad, Path, Rotation, Vehicle};
use crate::game::HomelandParams;
use crate::homeland_start::{HomelandEdge, HomelandStart, HomelandStartGen};
use crate::nation::{skin_colors, Nation, NationDescription};
use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{
    SendAvatars, SendGameState, SendNations, SendParameters, SendSettlements, SendWorld, Visibility,
};
use crate::world::World;
use commons::grid::Grid;
use commons::rand::prelude::*;
use commons::V2;
use isometric::Color;
use std::collections::HashMap;
use std::time::Duration;

const AVATAR_NAME: &str = "avatar";

pub struct SetupNewWorld<T> {
    tx: T,
}

impl<T> SetupNewWorld<T>
where
    T: SendAvatars
        + SendGameState
        + SendNations
        + SendParameters
        + SendSettlements
        + SendWorld
        + Visibility,
{
    pub fn new(tx: T) -> SetupNewWorld<T> {
        SetupNewWorld { tx }
    }

    pub async fn new_game(&self) {
        let (avatar_color, homeland_params, homeland_distance, nations, seed) = self
            .tx
            .send_parameters(|params| {
                (
                    params.avatar_color,
                    params.homeland.clone(),
                    params.homeland_distance,
                    params.nations.clone(),
                    params.seed,
                )
            })
            .await;
        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let homeland_starts = self.gen_homeland_starts(rng.clone(), homeland_params).await;

        let avatars = self
            .gen_avatar(
                homeland_starts[0].pre_landfall,
                avatar_color,
                avatar_skin_color(&mut rng),
            )
            .await;
        let nations = gen_nations(&mut rng, &nations, &homeland_starts.len());
        let homelands = self
            .gen_homelands(&homeland_distance, &homeland_starts, &nations)
            .await;

        join!(
            self.set_avatars(avatars),
            self.set_nations(nations),
            self.set_settlements(homelands)
        );

        self.set_visibility_from_voyage(&homeland_starts[0].voyage);
    }

    async fn gen_homeland_starts<R: Rng + Send + 'static>(
        &self,
        rng: R,
        params: HomelandParams,
    ) -> Vec<HomelandStart> {
        self.tx
            .send_world(move |world| gen_homeland_starts(rng, world, &params))
            .await
    }

    async fn gen_avatar(
        &self,
        position: V2<usize>,
        color: Color,
        skin_color: Color,
    ) -> HashMap<String, Avatar> {
        self.tx
            .send_world(move |world| gen_avatar(world, position, color, skin_color))
            .await
    }

    async fn gen_homelands(
        &self,
        homeland_distance: &Duration,
        homeland_starts: &[HomelandStart],
        nations: &HashMap<String, Nation>,
    ) -> HashMap<V2<usize>, Settlement> {
        let initial_population = self.initial_population(&nations.len()).await;
        gen_homelands(
            &homeland_distance,
            &homeland_starts,
            &nations,
            &initial_population,
        )
    }

    async fn initial_population(&self, homeland_count: &usize) -> f64 {
        let visible_land_positions = self
            .tx
            .send_game_state(|game_state| game_state.visible_land_positions)
            .await;
        visible_land_positions as f64 / *homeland_count as f64
    }

    async fn set_avatars(&self, new_avatars: HashMap<String, Avatar>) {
        self.tx
            .send_avatars(move |avatars| {
                avatars.all = new_avatars;
                avatars.selected = Some(AVATAR_NAME.to_string())
            })
            .await;
    }

    async fn set_nations(&self, new_nations: HashMap<String, Nation>) {
        self.tx
            .send_nations(move |nations| *nations = new_nations)
            .await;
    }

    async fn set_settlements(&self, new_settlements: HashMap<V2<usize>, Settlement>) {
        self.tx
            .send_settlements(move |settlements| *settlements = new_settlements)
            .await;
    }

    fn set_visibility_from_voyage(&self, voyage: &[V2<usize>]) {
        self.tx
            .check_visibility_and_reveal(voyage.iter().cloned().collect());
    }
}

fn gen_homeland_starts<R: Rng>(
    mut rng: R,
    world: &World,
    params: &HomelandParams,
) -> Vec<HomelandStart> {
    let min_distance_between_homelands =
        min_distance_between_homelands(world, params.count, &params.edges);
    let mut gen = HomelandStartGen::new(
        world,
        &mut rng,
        &params.edges,
        Some(min_distance_between_homelands),
    );
    (0..params.count).map(|_| gen.random_start()).collect()
}

fn min_distance_between_homelands(
    world: &World,
    homelands: usize,
    edges: &[HomelandEdge],
) -> usize {
    (total_edge_positions(world, edges) as f32 / (homelands as f32 * 2.0)).ceil() as usize
}

fn total_edge_positions(world: &World, edges: &[HomelandEdge]) -> usize {
    edges
        .iter()
        .map(|edge| match edge {
            HomelandEdge::North => world.width(),
            HomelandEdge::East => world.height(),
            HomelandEdge::South => world.width(),
            HomelandEdge::West => world.height(),
        })
        .sum()
}

fn avatar_skin_color<R: Rng>(rng: &mut R) -> Color {
    *skin_colors().choose(rng).unwrap()
}

fn gen_avatar(
    world: &World,
    position: V2<usize>,
    color: Color,
    skin_color: Color,
) -> HashMap<String, Avatar> {
    hashmap! {
        AVATAR_NAME.to_string() => Avatar {
            name: AVATAR_NAME.to_string(),
            path: Some(Path::stationary(
                world,
                position,
                Vehicle::Boat,
                Rotation::Up,
            )),
            color,
            skin_color,
            load: AvatarLoad::None,
        }
    }
}

fn gen_nations<R: Rng>(
    rng: &mut R,
    nations: &[NationDescription],
    count: &usize,
) -> HashMap<String, Nation> {
    nations
        .choose_multiple(rng, *count)
        .map(|nation| (nation.name.clone(), Nation::from_description(nation)))
        .collect()
}

fn gen_homelands(
    homeland_distance: &Duration,
    homeland_starts: &[HomelandStart],
    nations: &HashMap<String, Nation>,
    initial_population: &f64,
) -> HashMap<V2<usize>, Settlement> {
    nations
        .keys()
        .enumerate()
        .map(|(i, nation)| {
            gen_homeland(
                homeland_distance,
                &homeland_starts[i],
                nation.to_string(),
                *initial_population,
            )
        })
        .map(|settlement| (settlement.position, settlement))
        .collect()
}

fn gen_homeland(
    homeland_distance: &Duration,
    homeland_start: &HomelandStart,
    nation: String,
    initial_population: f64,
) -> Settlement {
    Settlement {
        class: SettlementClass::Homeland,
        position: homeland_start.homeland,
        name: nation.clone(),
        nation,
        current_population: initial_population,
        target_population: 0.0,
        gap_half_life: (*homeland_distance * 2).mul_f32(2.41), // 5.19 makes half life equivalent to '7/8th life'
        last_population_update_micros: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::M;

    #[test]
    fn test_min_distance_between_homelands() {
        let world = World::new(M::zeros(1024, 512), 0.5);
        let edges = vec![HomelandEdge::East, HomelandEdge::West];
        assert_eq!(min_distance_between_homelands(&world, 8, &edges), 64);
    }

    #[test]
    fn test_min_distance_between_homelands_rounds_up() {
        let world = World::new(M::zeros(1024, 512), 0.5);
        let edges = vec![HomelandEdge::East, HomelandEdge::West];
        assert_eq!(min_distance_between_homelands(&world, 9, &edges), 57);
    }
}
