use commons::index2d::*;
use commons::*;

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    controller: V2<usize>,
    position: V2<usize>,
    duration: Duration,
    since_micros: u128,
}

impl Ord for Claim {
    fn cmp(&self, other: &Self) -> Ordering {
        self.duration.cmp(&other.duration).then(
            self.since_micros.cmp(&other.since_micros).then(
                self.controller
                    .x
                    .cmp(&other.controller.x)
                    .then(self.controller.y.cmp(&other.controller.y)),
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
    pub fn new<T>(grid: &dyn Grid<T>) -> Territory {
        Territory {
            territory: HashMap::new(),
            claims: Vec2D::same_size_as(grid, HashMap::new()),
        }
    }

    pub fn set_durations(
        &mut self,
        controller: V2<usize>,
        durations: &HashMap<V2<usize>, Duration>,
        game_micros: &u128,
    ) -> Vec<TerritoryChange> {
        let changes = self.update_claims(&controller, &durations, game_micros);
        self.territory
            .insert(controller, durations.keys().cloned().collect());
        changes
    }

    pub fn who_claims(&self, position: &V2<usize>) -> HashSet<V2<usize>> {
        self.claims
            .get(position)
            .map(|map| map.values().map(|claim| claim.controller).collect())
            .unwrap_or_default()
    }

    pub fn who_controls(&self, position: &V2<usize>) -> Option<V2<usize>> {
        self.claims.get(position).ok().and_then(|map| {
            map.values()
                .min_by(|a, b| a.cmp(&b))
                .map(|claim| claim.controller)
        })
    }

    pub fn who_controls_tile<T>(
        &self,
        grid: &dyn Grid<T>,
        position: &V2<usize>,
    ) -> Option<V2<usize>> {
        grid.get_corners_in_bounds(position)
            .iter()
            .flat_map(|corner| self.claims.get(corner).unwrap().values())
            .min_by(|a, b| a.cmp(&b))
            .map(|claim| claim.controller)
    }

    pub fn controllers(&self) -> HashSet<V2<usize>> {
        self.territory.keys().cloned().collect()
    }

    #[allow(dead_code)]
    pub fn claimed_territory(&self, controller: &V2<usize>) -> Option<&HashSet<V2<usize>>> {
        self.territory.get(controller)
    }

    pub fn controlled_territory(&self, controller: &V2<usize>) -> HashSet<V2<usize>> {
        self.territory
            .get(controller)
            .map(|territory| {
                territory
                    .iter()
                    .filter(|position| self.who_controls(position) == Some(*controller))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
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
            let previous_controller = self.who_controls(&position);
            if let Ok(map) = self.claims.get_mut(&position) {
                map.remove(controller);
            }
            let new_controller = self.who_controls(&position);
            out.append(&mut Self::get_territory_changes(
                position,
                previous_controller,
                new_controller,
            ));
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
            let previous_controller = self.who_controls(position);
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

            let new_controller = self.who_controls(position);
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

    fn grid() -> M<u8> {
        M::zeros(3, 3)
    }

    fn territory() -> Territory {
        Territory::new::<u8>(&grid())
    }

    #[test]
    fn new_controller_changes() {
        let changes = territory().set_durations(
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
    fn no_longer_claimed_no_other_claim_changes() {
        let mut territory = territory();
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
    fn who_claims_no_claims() {
        assert_eq!(territory().who_claims(&v2(0, 0)), HashSet::new());
    }

    #[test]
    fn who_claims_single_claim() {
        let mut territory = territory();
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        assert_eq!(
            territory.who_claims(&v2(0, 0)),
            [v2(1, 1)].iter().cloned().collect()
        );
    }

    #[test]
    fn who_claims_multiple_claims() {
        let mut territory = territory();
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
            &0,
        );
        assert_eq!(
            territory.who_claims(&v2(0, 0)),
            [v2(1, 1), v2(2, 2)].iter().cloned().collect()
        );
    }

    #[test]
    fn who_controls_no_claims() {
        assert_eq!(territory().who_controls(&v2(0, 0)), None);
    }

    #[test]
    fn who_controls_single_claim() {
        let mut territory = territory();
        territory.set_durations(
            v2(1, 1),
            &[(v2(0, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &0,
        );
        assert_eq!(territory.who_controls(&v2(0, 0)), Some(v2(1, 1)));
    }

    #[test]
    fn who_controls_multiple_claims_different_durations() {
        let mut territory = territory();
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
        assert_eq!(territory.who_controls(&v2(0, 0)), Some(v2(2, 2)));
    }

    #[test]
    fn who_controls_multiple_claims_same_durations() {
        let mut territory = territory();
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
        assert_eq!(territory.who_controls(&v2(0, 0)), Some(v2(1, 1)));
    }

    #[test]
    fn who_controls_tile_no_claims() {
        assert_eq!(territory().who_controls_tile(&grid(), &v2(0, 0)), None);
    }

    #[test]
    fn who_controls_tile_single_claim() {
        let mut territory = territory();
        territory.set_durations(
            v2(1, 1),
            &[
                (v2(0, 0), Duration::from_millis(2)),
                (v2(1, 0), Duration::from_millis(2)),
            ]
            .iter()
            .cloned()
            .collect(),
            &0,
        );
        assert_eq!(
            territory.who_controls_tile(&grid(), &v2(0, 0)),
            Some(v2(1, 1))
        );
    }

    #[test]
    fn who_controls_tile_multiple_claims_different_durations() {
        let mut territory = territory();
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
            &[(v2(1, 0), Duration::from_millis(1))]
                .iter()
                .cloned()
                .collect(),
            &1,
        );
        assert_eq!(
            territory.who_controls_tile(&grid(), &v2(0, 0)),
            Some(v2(2, 2))
        );
    }

    #[test]
    fn who_controls_tile_multiple_claims_same_durations() {
        let mut territory = territory();
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
            &[(v2(1, 0), Duration::from_millis(2))]
                .iter()
                .cloned()
                .collect(),
            &1,
        );
        assert_eq!(
            territory.who_controls_tile(&grid(), &v2(0, 0)),
            Some(v2(1, 1))
        );
    }

    #[test]
    fn claimed_territory() {
        let mut territory = territory();
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
            territory.claimed_territory(&v2(1, 1)),
            Some(&[v2(0, 0)].iter().cloned().collect())
        );
    }

    #[test]
    fn claimed_territory_no_claims() {
        assert_eq!(territory().claimed_territory(&v2(1, 1)), None);
    }

    #[test]
    fn controllers() {
        let mut territory = territory();
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
            territory.controllers(),
            [v2(1, 1), v2(2, 2)].iter().cloned().collect()
        );
    }

    #[test]
    fn controllers_no_controllers() {
        assert_eq!(territory().controllers(), HashSet::new());
    }

    #[test]
    fn controlled_territory_no_claims() {
        assert_eq!(territory().controlled_territory(&v2(1, 1)), HashSet::new());
    }

    #[test]
    fn controlled_territory_shows_controlled() {
        let mut territory = territory();
        territory.set_durations(
            v2(1, 1),
            &[
                (v2(0, 0), Duration::from_millis(2)),
                (v2(1, 0), Duration::from_millis(2)),
            ]
            .iter()
            .cloned()
            .collect(),
            &0,
        );
        assert_eq!(
            territory.controlled_territory(&v2(1, 1)),
            [v2(0, 0), v2(1, 0)].iter().cloned().collect()
        );
    }

    #[test]
    fn controlled_territory_does_not_show_claimed_but_not_controlled() {
        let mut territory = territory();
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
        assert_eq!(territory.controlled_territory(&v2(1, 1)), HashSet::new());
    }
}
