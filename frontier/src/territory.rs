use commons::grid::Grid;
use commons::index2d::*;
use commons::*;

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub type Controllers = M<Option<V2<usize>>>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub controller: V2<usize>,
    pub position: V2<usize>,
    pub duration: Duration,
    pub since_micros: u128,
}

impl Ord for Claim {
    fn cmp(&self, other: &Self) -> Ordering {
        self.duration.cmp(&other.duration).then(
            self.since_micros.cmp(&other.since_micros).then(
                self.controller.x.cmp(&other.controller.x).then(
                    self.controller
                        .y
                        .cmp(&other.controller.y)
                        .then(self.position.x.cmp(&other.position.x))
                        .then(self.position.y.cmp(&other.position.y)),
                ),
            ),
        )
    }
}

impl PartialOrd for Claim {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(PartialEq, Debug)]
pub struct TerritoryChange {
    pub controller: V2<usize>,
    pub position: V2<usize>,
    pub controlled: bool,
}

impl TerritoryChange {
    pub fn gain(controller: V2<usize>, position: V2<usize>) -> TerritoryChange {
        TerritoryChange {
            controller,
            position,
            controlled: true,
        }
    }

    pub fn loss(controller: V2<usize>, position: V2<usize>) -> TerritoryChange {
        TerritoryChange {
            controller,
            position,
            controlled: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Territory {
    territory: HashMap<V2<usize>, HashSet<V2<usize>>>,
    claims: Vec2D<HashMap<V2<usize>, Claim>>,
}

impl Territory {
    pub fn new(width: usize, height: usize) -> Territory {
        Territory {
            territory: HashMap::new(),
            claims: Vec2D::new(width, height, HashMap::new()),
        }
    }

    pub fn add_controller(&mut self, controller: V2<usize>) {
        self.territory.insert(controller, HashSet::new());
    }

    pub fn remove_controller(&mut self, controller: &V2<usize>) {
        self.territory.remove(controller);
    }

    pub fn set_durations(
        &mut self,
        controller: V2<usize>,
        durations: &HashMap<V2<usize>, Duration>,
        game_micros: &u128,
    ) -> Vec<TerritoryChange> {
        if !self.territory.contains_key(&controller) {
            return vec![];
        }
        let changes = self.update_claims(&controller, &durations, game_micros);
        self.territory
            .insert(controller, durations.keys().cloned().collect());
        changes
    }

    pub fn who_controls_tile(&self, position: &V2<usize>) -> Option<&Claim> {
        self.claims
            .get_corners_in_bounds(position)
            .iter()
            .flat_map(|corner| self.claims.get(corner))
            .flat_map(|map| map.values())
            .min_by(|a, b| a.cmp(&b))
    }

    pub fn who_controls(&self, position: &V2<usize>) -> Option<&Claim> {
        self.claims
            .get(position)
            .ok()
            .and_then(|map| map.values().min_by(|a, b| a.cmp(&b)))
    }

    pub fn anyone_controls(&self, position: &V2<usize>) -> bool {
        self.claims
            .get(position)
            .map_or(false, |claims| !claims.is_empty())
    }

    pub fn is_controlled_by(&self, position: &V2<usize>, controller: &V2<usize>) -> bool {
        matches!(
            self.who_controls(position),
            Some(Claim {
                controller: controlled_by,
                ..
            }) if controlled_by == controller
        )
    }

    pub fn controlled(&self, controller: &V2<usize>) -> HashSet<V2<usize>> {
        let claims: &HashSet<V2<usize>> =
            unwrap_or!(self.territory.get(controller), return HashSet::new());
        claims
            .iter()
            .cloned()
            .filter(|position| self.is_controlled_by(position, controller))
            .collect()
    }

    fn update_claims(
        &mut self,
        controller: &V2<usize>,
        durations: &HashMap<V2<usize>, Duration>,
        game_micros: &u128,
    ) -> Vec<TerritoryChange> {
        let mut out = vec![];
        out.append(
            &mut self.remove_claims(controller, self.get_claims_to_remove(controller, durations)),
        );
        out.append(&mut self.add_claims(controller, durations, game_micros));
        out
    }

    fn get_claims_to_remove(
        &self,
        controller: &V2<usize>,
        durations: &HashMap<V2<usize>, Duration>,
    ) -> Vec<V2<usize>> {
        self.territory
            .get(controller)
            .unwrap_or(&HashSet::new())
            .iter()
            .cloned()
            .filter(|position| !durations.contains_key(position))
            .collect()
    }

    fn remove_claims(
        &mut self,
        controller: &V2<usize>,
        claims: Vec<V2<usize>>,
    ) -> Vec<TerritoryChange> {
        let mut out = vec![];
        for position in claims {
            let previous_controller = self.who_controls(&position).map(|claim| claim.controller);
            if let Ok(map) = self.claims.get_mut(&position) {
                map.remove(controller);
                let new_controller = self.who_controls(&position).map(|claim| claim.controller);
                out.append(&mut Self::get_territory_changes(
                    position,
                    previous_controller,
                    new_controller,
                ));
            }
        }
        out
    }

    fn add_claims(
        &mut self,
        controller: &V2<usize>,
        durations: &HashMap<V2<usize>, Duration>,
        game_micros: &u128,
    ) -> Vec<TerritoryChange> {
        let mut out = vec![];
        for (position, duration) in durations {
            let previous_controller = self.who_controls(position).map(|claim| claim.controller);
            let claims = self.claims.get_mut(&position).unwrap();

            let since_micros = match claims.get(controller) {
                Some(claim) => claim.since_micros,
                None => *game_micros,
            };

            claims.insert(
                *controller,
                Claim {
                    controller: *controller,
                    position: *position,
                    duration: *duration,
                    since_micros,
                },
            );

            let new_controller = self.who_controls(position).map(|claim| claim.controller);
            out.append(&mut Self::get_territory_changes(
                *position,
                previous_controller,
                new_controller,
            ));
        }

        out
    }

    fn get_territory_changes(
        position: V2<usize>,
        previous_controller: Option<V2<usize>>,
        new_controller: Option<V2<usize>>,
    ) -> Vec<TerritoryChange> {
        let mut out = vec![];
        if previous_controller != new_controller {
            if let Some(controller) = previous_controller {
                out.push(TerritoryChange::loss(controller, position))
            }
            if let Some(controller) = new_controller {
                out.push(TerritoryChange::gain(controller, position))
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn territory() -> Territory {
        Territory::new(3, 3)
    }

    #[test]
    fn new_controller_changes() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        let changes = territory.set_durations(
            v2(1, 1),
            &[
                (v2(1, 0), Duration::from_millis(1)),
                (v2(1, 1), Duration::from_millis(0)),
            ]
            .iter()
            .cloned()
            .collect(),
            &0,
        );
        assert!(same_elements(
            &changes,
            &[
                TerritoryChange::gain(v2(1, 1), v2(1, 0)),
                TerritoryChange::gain(v2(1, 1), v2(1, 1))
            ]
        ));
    }

    #[test]
    fn add_controller() {
        let mut territory = territory();
        territory.add_controller(v2(0, 0));
        assert_eq!(
            territory
                .territory
                .keys()
                .cloned()
                .collect::<HashSet<V2<usize>>>(),
            hashset! {v2(0, 0)}
        );
    }

    #[test]
    fn remove_controller() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.remove_controller(&v2(1, 1));
        assert_eq!(territory.territory.keys().count(), 0);
    }

    #[test]
    fn no_longer_claimed_no_other_claim_changes() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.set_durations(
            v2(1, 1),
            &[
                (v2(1, 0), Duration::from_millis(1)),
                (v2(1, 1), Duration::from_millis(0)),
            ]
            .iter()
            .cloned()
            .collect(),
            &0,
        );
        let changes = territory.set_durations(v2(1, 1), &HashMap::new(), &0);
        assert!(same_elements(
            &changes,
            &[
                TerritoryChange::loss(v2(1, 1), v2(1, 0)),
                TerritoryChange::loss(v2(1, 1), v2(1, 1))
            ]
        ));
    }

    #[test]
    fn no_longer_claimed_other_claim_changes() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.add_controller(v2(2, 2));
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(1))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        territory.set_durations(
            v2(2, 2),
            &[(v2(0, 0), Duration::from_millis(1))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        let changes = territory.set_durations(v2(1, 1), &HashMap::new(), &0);
        assert!(same_elements(
            &changes,
            &[
                TerritoryChange::loss(v2(1, 1), v2(0, 0)),
                TerritoryChange::gain(v2(2, 2), v2(0, 0))
            ]
        ));
    }

    #[test]
    fn additional_claim_changes() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.set_durations(
            v2(1, 1),
            &[(v2(1, 0), Duration::from_millis(1))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        let changes = territory.set_durations(
            v2(1, 1),
            &[
                (v2(1, 0), Duration::from_millis(1)),
                (v2(1, 1), Duration::from_millis(1)),
            ]
            .iter()
            .cloned()
            .collect(),
            &0,
        );
        assert!(same_elements(
            &changes,
            &[TerritoryChange::gain(v2(1, 1), v2(1, 1),)]
        ));
    }

    #[test]
    fn owner_change_through_duration_decrease_changes() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.add_controller(v2(2, 2));
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        territory.set_durations(
            v2(2, 2),
            &[(v2(0, 0), Duration::from_millis(3))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        let changes = territory.set_durations(
            v2(2, 2),
            &[(v2(0, 0), Duration::from_millis(1))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        assert!(same_elements(
            &changes,
            &[
                TerritoryChange::loss(v2(1, 1), v2(0, 0)),
                TerritoryChange::gain(v2(2, 2), v2(0, 0))
            ]
        ));
    }

    #[test]
    fn owner_change_through_duration_increase_changes() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.add_controller(v2(2, 2));
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        territory.set_durations(
            v2(2, 2),
            &[(v2(0, 0), Duration::from_millis(3))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        let changes = territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(4))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        assert!(same_elements(
            &changes,
            &[
                TerritoryChange::loss(v2(1, 1), v2(0, 0)),
                TerritoryChange::gain(v2(2, 2), v2(0, 0))
            ]
        ));
    }

    #[test]
    fn who_controls_no_claims() {
        assert_eq!(territory().who_controls(&v2(0, 0)), None);
        assert_eq!(territory().anyone_controls(&v2(0, 0)), false);
    }

    #[test]
    fn who_controls_single_claim() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        assert_eq!(
            territory.who_controls(&v2(0, 0)),
            Some(&Claim {
                controller: v2(1, 1),
                position: v2(0, 0),
                duration: Duration::from_millis(2),
                since_micros: 0
            })
        );
        assert_eq!(territory.anyone_controls(&v2(0, 0)), true);
    }

    #[test]
    fn who_controls_multiple_claims_different_durations() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.add_controller(v2(2, 2));
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        territory.set_durations(
            v2(2, 2),
            &[(v2(0, 0), Duration::from_millis(1))]
                .iter()
                .cloned()
                .collect(),
            &1,
        );
        assert_eq!(
            territory.who_controls(&v2(0, 0)),
            Some(&Claim {
                controller: v2(2, 2),
                position: v2(0, 0),
                duration: Duration::from_millis(1),
                since_micros: 1
            })
        );
        assert_eq!(territory.anyone_controls(&v2(0, 0)), true);
    }

    #[test]
    fn who_controls_multiple_claims_same_durations() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.add_controller(v2(2, 2));
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        territory.set_durations(
            v2(2, 2),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &1,
        );
        assert_eq!(
            territory.who_controls(&v2(0, 0)),
            Some(&Claim {
                controller: v2(1, 1),
                position: v2(0, 0),
                duration: Duration::from_millis(2),
                since_micros: 0
            })
        );
        assert_eq!(territory.anyone_controls(&v2(0, 0)), true);
    }

    #[test]
    fn who_controls_tile_no_claims() {
        let territory = territory();
        assert_eq!(territory.who_controls_tile(&v2(0, 0)), None);
    }

    #[test]
    fn who_controls_tile_not_all_corners_claimed() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        assert_eq!(
            territory.who_controls(&v2(0, 0)),
            Some(&Claim {
                controller: v2(1, 1),
                position: v2(0, 0),
                duration: Duration::from_millis(2),
                since_micros: 0
            })
        );
    }

    #[test]
    fn who_controls_tile_all_corners_claimed() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.set_durations(
            v2(1, 1),
            &[
                (v2(0, 0), Duration::from_millis(2)),
                (v2(1, 0), Duration::from_millis(2)),
                (v2(1, 1), Duration::from_millis(2)),
                (v2(0, 1), Duration::from_millis(2)),
            ]
            .iter()
            .cloned()
            .collect(),
            &0,
        );
        assert_eq!(
            territory.who_controls_tile(&v2(0, 0)),
            Some(&Claim {
                controller: v2(1, 1),
                position: v2(0, 0),
                duration: Duration::from_millis(2),
                since_micros: 0
            })
        );
    }

    #[test]
    fn who_controls_tile_multiple_claims_different_durations() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.add_controller(v2(2, 2));
        territory.set_durations(
            v2(1, 1),
            &[
                (v2(0, 0), Duration::from_millis(2)),
                (v2(1, 0), Duration::from_millis(2)),
                (v2(1, 1), Duration::from_millis(2)),
                (v2(0, 1), Duration::from_millis(2)),
            ]
            .iter()
            .cloned()
            .collect(),
            &0,
        );
        territory.set_durations(
            v2(2, 2),
            &[
                (v2(0, 0), Duration::from_millis(1)),
                (v2(1, 0), Duration::from_millis(1)),
                (v2(1, 1), Duration::from_millis(1)),
                (v2(0, 1), Duration::from_millis(1)),
            ]
            .iter()
            .cloned()
            .collect(),
            &1,
        );
        assert_eq!(
            territory.who_controls_tile(&v2(0, 0)),
            Some(&Claim {
                controller: v2(2, 2),
                position: v2(0, 0),
                duration: Duration::from_millis(1),
                since_micros: 1
            })
        );
    }

    #[test]
    fn who_controls_tile_multiple_claims_same_durations() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.add_controller(v2(2, 2));
        territory.set_durations(
            v2(1, 1),
            &[
                (v2(0, 0), Duration::from_millis(2)),
                (v2(1, 0), Duration::from_millis(2)),
                (v2(1, 1), Duration::from_millis(2)),
                (v2(0, 1), Duration::from_millis(2)),
            ]
            .iter()
            .cloned()
            .collect(),
            &0,
        );
        territory.set_durations(
            v2(2, 2),
            &[
                (v2(0, 0), Duration::from_millis(2)),
                (v2(1, 0), Duration::from_millis(2)),
                (v2(1, 1), Duration::from_millis(2)),
                (v2(0, 1), Duration::from_millis(2)),
            ]
            .iter()
            .cloned()
            .collect(),
            &1,
        );
        assert_eq!(
            territory.who_controls_tile(&v2(0, 0)),
            Some(&Claim {
                controller: v2(1, 1),
                position: v2(0, 0),
                duration: Duration::from_millis(2),
                since_micros: 0
            })
        );
    }

    #[test]
    fn is_controlled_by_returns_true_when_controlled() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.set_durations(
            v2(1, 1),
            &hashmap! {v2(0, 0) => Duration::from_millis(2)},
            &0,
        );
        assert!(territory.is_controlled_by(&v2(0, 0), &v2(1, 1)));
    }

    #[test]
    fn is_controlled_by_returns_false_when_not_controlled() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        assert!(!territory.is_controlled_by(&v2(0, 0), &v2(1, 1)));
    }

    #[test]
    fn controlled_no_claims() {
        assert_eq!(territory().controlled(&v2(1, 1)), hashset! {});
    }

    #[test]
    fn controlled_includes_controlled() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.set_durations(
            v2(1, 1),
            &hashmap! {v2(0, 0) => Duration::from_millis(2)},
            &0,
        );
        assert_eq!(territory.controlled(&v2(1, 1)), hashset! {v2(0, 0)},);
    }

    #[test]
    fn controlled_does_not_includes_claimed_but_not_controlled() {
        let mut territory = territory();
        territory.add_controller(v2(1, 1));
        territory.set_durations(
            v2(1, 1),
            &hashmap! {v2(0, 0) => Duration::from_millis(2)},
            &0,
        );
        territory.add_controller(v2(2, 2));
        territory.set_durations(
            v2(2, 2),
            &hashmap! {v2(0, 0) => Duration::from_millis(1)},
            &0,
        );
        assert_eq!(territory.controlled(&v2(1, 1)), hashset! {});
    }
}
