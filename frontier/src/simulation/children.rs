use super::*;
use crate::game_event_consumers::FARM_CANDIDATE_TARGETS;

use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};
use std::default::Default;

const HANDLE: &str = "children_sim";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChildrenParams {
    childbearing_age_start_inclusive: u128,
    childbearing_age_end_exclusive: u128,
    probability_per_year: f64,
}

impl ChildrenParams {
    fn child_probability(&self, birthday: &u128, year: &u128) -> f64 {
        let childbearing_start_year = birthday + self.childbearing_age_start_inclusive;
        let childbearing_end_year = birthday + self.childbearing_age_end_exclusive;
        if year < &childbearing_start_year || year >= &childbearing_end_year {
            0.0
        } else {
            self.probability_per_year
        }
    }
}

impl Default for ChildrenParams {
    fn default() -> ChildrenParams {
        ChildrenParams {
            childbearing_age_start_inclusive: 18,
            childbearing_age_end_exclusive: 40,
            probability_per_year: 2.0 / (40.0 - 18.0),
        }
    }
}

struct Child {
    birthday: u128,
    birthplace: V2<usize>,
}

struct Parent {
    birthday: u128,
    farm: V2<usize>,
}

pub struct ChildrenSim {
    params: ChildrenParams,
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
    rng: SmallRng,
}

impl Step for ChildrenSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn step(&mut self, year: u128) {
        block_on(self.step_async(year))
    }
}

impl ChildrenSim {
    pub fn new(
        params: ChildrenParams,
        seed: u64,
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
    ) -> ChildrenSim {
        ChildrenSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
            params,
            rng: SeedableRng::seed_from_u64(seed),
        }
    }

    async fn step_async(&mut self, year: u128) {
        for parent in self.get_parents().await {
            self.step_parent(&year, parent).await
        }
    }

    async fn get_parents(&mut self) -> Vec<Parent> {
        self.game_tx
            .update(|game| {
                game.game_state()
                    .citizens
                    .values()
                    .flat_map(|citizen| as_parent(citizen))
                    .collect()
            })
            .await
    }

    async fn step_parent(&mut self, year: &u128, parent: Parent) {
        let child = match self.get_child(year, &parent).await {
            Some(child) => child,
            _ => return,
        };
        let farm = match self.get_farm(child.birthplace).await {
            Some(farm) => farm,
            _ => return,
        };
        if self.add_child(child, farm).await {
            self.remove_candidate(farm).await;
        }
    }

    async fn get_child(&mut self, year: &u128, parent: &Parent) -> Option<Child> {
        if !self.having_child(year, &parent.birthday) {
            return None;
        }
        let child = Child {
            birthday: *year,
            birthplace: parent.farm,
        };
        Some(child)
    }

    fn having_child(&mut self, year: &u128, birthday: &u128) -> bool {
        let r: f64 = self.rng.gen_range(0.0, 1.0);
        let p = self.params.child_probability(birthday, year);
        r <= p
    }

    async fn get_farm(&mut self, position: V2<usize>) -> Option<V2<usize>> {
        self.pathfinder_tx
            .update(move |service| {
                let mut candidates = service
                    .pathfinder()
                    .closest_targets(&[position], FARM_CANDIDATE_TARGETS);
                candidates.pop()
            })
            .await
    }

    async fn add_child(&mut self, child: Child, farm: V2<usize>) -> bool {
        let rotate_farm = self.rng.gen();
        self.game_tx
            .update(move |game| add_child(game, child, farm, rotate_farm))
            .await
    }

    async fn remove_candidate(&mut self, farm: V2<usize>) {
        self.pathfinder_tx
            .update(move |service| {
                service
                    .pathfinder()
                    .load_target(FARM_CANDIDATE_TARGETS, &farm, false)
            })
            .await
    }
}

fn add_child(game: &mut Game, child: Child, farm: V2<usize>, rotate_farm: bool) -> bool {
    if !game.update_object(
        WorldObject::Farm {
            rotated: rotate_farm,
        },
        farm,
        true,
    ) {
        return false;
    }
    let game_state = game.mut_state();
    let citizen = Citizen {
        name: game_state.citizens.len().to_string(),
        birthday: child.birthday,
        birthplace: child.birthplace,
        farm: Some(farm),
    };
    game_state.citizens.insert(citizen.name.clone(), citizen);
    true
}

fn as_parent(citizen: &Citizen) -> Option<Parent> {
    citizen.farm.map(|farm| Parent {
        birthday: citizen.birthday,
        farm,
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::Almost;

    #[test]
    fn child_probability_year_inside_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start_inclusive: 100,
            childbearing_age_end_exclusive: 200,
            probability_per_year: 1.0,
        }
        .child_probability(&0, &150)
        .almost(1.0));
    }
    #[test]
    fn child_probability_year_inside_childbearing_range_non_zero_birthday() {
        assert!(ChildrenParams {
            childbearing_age_start_inclusive: 100,
            childbearing_age_end_exclusive: 200,
            probability_per_year: 1.0,
        }
        .child_probability(&100, &250)
        .almost(1.0));
    }
    #[test]
    fn child_probability_year_equals_start_of_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start_inclusive: 100,
            childbearing_age_end_exclusive: 200,
            probability_per_year: 1.0,
        }
        .child_probability(&0, &100)
        .almost(1.0));
    }
    #[test]
    fn child_probability_year_equals_end_of_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start_inclusive: 100,
            childbearing_age_end_exclusive: 200,
            probability_per_year: 1.0,
        }
        .child_probability(&0, &200)
        .almost(0.0));
    }
    #[test]
    fn child_probability_year_before_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start_inclusive: 100,
            childbearing_age_end_exclusive: 200,
            probability_per_year: 1.0,
        }
        .child_probability(&0, &99)
        .almost(0.0));
    }
    #[test]
    fn child_probability_year_after_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start_inclusive: 100,
            childbearing_age_end_exclusive: 200,
            probability_per_year: 1.0,
        }
        .child_probability(&0, &201)
        .almost(0.0));
    }
}
