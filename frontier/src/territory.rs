use commons::index2d::*;
use commons::*;

use serde::{Deserialize, Serialize};
use std::collections::hash_map::Iter;
use std::collections::{HashMap, HashSet};

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

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Territory {
    index: Index2D,
    territory: HashMap<V2<usize>, HashSet<V2<usize>>>,
    control: Vec<HashSet<V2<usize>>>,
}

impl Territory {
    pub fn new<T>(grid: &dyn Grid<T>) -> Territory {
        let index = Index2D::new(grid.width(), grid.height());
        Territory {
            index,
            territory: HashMap::new(),
            control: vec![HashSet::new(); index.indices()],
        }
    }

    fn compute_changes(
        &self,
        controller: &V2<usize>,
        territory: &HashSet<V2<usize>>,
    ) -> Vec<TerritoryChange> {
        let empty = HashSet::new();
        let previous_territory = match self.territory.get(controller) {
            Some(territory) => territory,
            None => &empty,
        };

        let gains = territory
            .difference(previous_territory)
            .cloned()
            .map(|position| TerritoryChange::gain(*controller, position));
        let losses = previous_territory
            .difference(&territory)
            .cloned()
            .map(|position| TerritoryChange::loss(*controller, position));

        gains.chain(losses).collect()
    }

    fn apply_changes(&mut self, controller: &V2<usize>, changes: &[TerritoryChange]) {
        for change in changes {
            let index = self.index.get_index(&change.position).unwrap();
            if change.controlled {
                self.control[index].insert(*controller);
            } else {
                self.control[index].remove(controller);
            }
        }
    }

    pub fn control(
        &mut self,
        controller: V2<usize>,
        territory: HashSet<V2<usize>>,
    ) -> Vec<TerritoryChange> {
        let changes = self.compute_changes(&controller, &territory);
        self.territory.insert(controller, territory);
        self.apply_changes(&controller, &changes);
        changes
    }

    pub fn who_controls(&self, position: &V2<usize>) -> &HashSet<V2<usize>> {
        &self.control[self.index.get_index(position).unwrap()]
    }

    pub fn controllers(&self) -> Iter<V2<usize>, HashSet<V2<usize>>> {
        self.territory.iter()
    }

    pub fn all_corners_controlled<T>(&self, grid: &dyn Grid<T>, position: &V2<usize>) -> bool {
        grid.get_corners(position)
            .iter()
            .filter(|corner| grid.in_bounds(corner))
            .all(|corner| !self.who_controls(corner).is_empty())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn new_controller() {
        let mut territory = Territory::new::<u8>(&M::zeros(3, 3));
        let changes = territory.control(v2(1, 1), [v2(1, 0), v2(1, 1)].iter().cloned().collect());
        assert!(same_elements(
            &changes,
            &[
                TerritoryChange::gain(v2(1, 1), v2(1, 0)),
                TerritoryChange::gain(v2(1, 1), v2(1, 1))
            ]
        ));
        assert_eq!(
            territory.who_controls(&v2(1, 0)),
            &[v2(1, 1)].iter().cloned().collect()
        );
        assert_eq!(
            territory.territory[&v2(1, 1)],
            [v2(1, 0), v2(1, 1)].iter().cloned().collect()
        );
    }

    #[test]
    fn lose_all() {
        let mut territory = Territory::new::<u8>(&M::zeros(3, 3));
        territory.control(v2(1, 1), [v2(1, 0), v2(1, 1)].iter().cloned().collect());
        let changes = territory.control(v2(1, 1), HashSet::new());
        assert!(same_elements(
            &changes,
            &[
                TerritoryChange::loss(v2(1, 1), v2(1, 0)),
                TerritoryChange::loss(v2(1, 1), v2(1, 1))
            ]
        ));
        assert_eq!(territory.who_controls(&v2(1, 0)), &HashSet::new());
        assert_eq!(territory.territory[&v2(1, 1)], HashSet::new());
    }

    #[test]
    fn gain_some() {
        let mut territory = Territory::new::<u8>(&M::zeros(3, 3));
        territory.control(v2(1, 1), [v2(1, 0)].iter().cloned().collect());
        let changes = territory.control(v2(1, 1), [v2(1, 0), v2(1, 1)].iter().cloned().collect());
        assert!(same_elements(
            &changes,
            &[TerritoryChange::gain(v2(1, 1), v2(1, 1))]
        ));
        assert_eq!(
            territory.who_controls(&v2(1, 0)),
            &[v2(1, 1)].iter().cloned().collect()
        );
        assert_eq!(
            territory.territory[&v2(1, 1)],
            [v2(1, 0), v2(1, 1)].iter().cloned().collect()
        );
    }

    #[test]
    fn mix() {
        let mut territory = Territory::new::<u8>(&M::zeros(3, 3));
        territory.control(v2(1, 1), [v2(1, 0)].iter().cloned().collect());
        let changes = territory.control(v2(1, 1), [v2(1, 1)].iter().cloned().collect());
        assert!(same_elements(
            &changes,
            &[
                TerritoryChange::loss(v2(1, 1), v2(1, 0)),
                TerritoryChange::gain(v2(1, 1), v2(1, 1))
            ]
        ));
        assert_eq!(territory.who_controls(&v2(1, 0)), &HashSet::new());
        assert_eq!(
            territory.territory[&v2(1, 1)],
            [v2(1, 1)].iter().cloned().collect()
        );
    }

    #[test]
    fn multiple_ownership() {
        let mut territory = Territory::new::<u8>(&M::zeros(3, 3));
        territory.control(v2(0, 0), [v2(1, 0)].iter().cloned().collect());
        territory.control(v2(1, 1), [v2(1, 0)].iter().cloned().collect());
        assert_eq!(
            territory.who_controls(&v2(1, 0)),
            &[v2(0, 0), v2(1, 1)].iter().cloned().collect()
        );
        assert_eq!(
            territory.territory[&v2(0, 0)],
            [v2(1, 0)].iter().cloned().collect()
        );
        assert_eq!(
            territory.territory[&v2(1, 1)],
            [v2(1, 0)].iter().cloned().collect()
        );
    }

    #[test]
    fn all_corners_controlled() {
        let mut territory = Territory::new::<u8>(&M::zeros(3, 3));
        territory.control(
            v2(0, 0),
            [v2(0, 0), v2(1, 0), v2(1, 1), v2(0, 1)]
                .iter()
                .cloned()
                .collect(),
        );
        assert!(territory.all_corners_controlled::<u8>(&M::zeros(3, 3), &v2(0, 0)));
        assert!(!territory.all_corners_controlled::<u8>(&M::zeros(3, 3), &v2(1, 0)));
        assert!(!territory.all_corners_controlled::<u8>(&M::zeros(3, 3), &v2(1, 1)));
        assert!(!territory.all_corners_controlled::<u8>(&M::zeros(3, 3), &v2(0, 1)));
    }
}
