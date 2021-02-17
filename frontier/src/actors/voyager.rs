use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{RevealPositions, WithSettlements, WithWorld};
use crate::world::World;
use commons::grid::Grid;
use commons::{v2, V2};
use line_drawing::WalkGrid;
use std::collections::{HashMap, HashSet};

const NAME: &str = "voyager";

pub struct Voyager<T> {
    tx: T,
}

impl<T> Voyager<T>
where
    T: RevealPositions + WithSettlements + WithWorld + Send,
{
    pub fn new(tx: T) -> Voyager<T> {
        Voyager { tx }
    }

    pub async fn voyage_to(&mut self, positions: HashSet<V2<usize>>, by: &'static str) {
        if by == NAME {
            return;
        } // avoid chain reaction
        let (positions, homelands) =
            join!(self.filter_coastal(positions), self.homeland_positions());
        let to_reveal = self.get_positions_to_reveal(&positions, &homelands).await;
        self.tx.reveal_positions(&to_reveal, NAME).await;
    }

    async fn filter_coastal(&self, positions: HashSet<V2<usize>>) -> HashSet<V2<usize>>
    where
        T: WithWorld,
    {
        self.tx
            .with_world(|world| filter_coastal(world, positions))
            .await
    }

    async fn homeland_positions(&self) -> HashSet<V2<usize>> {
        self.tx
            .with_settlements(|settlements| homeland_positions(settlements))
            .await
    }

    async fn get_positions_to_reveal(
        &self,
        positions: &HashSet<V2<usize>>,
        homelands: &HashSet<V2<usize>>,
    ) -> HashSet<V2<usize>> {
        self.tx
            .with_world(|world| {
                homelands
                    .iter()
                    .flat_map(|homeland| {
                        positions.iter().flat_map(move |position| {
                            get_positions_to_reveal(world, homeland, position)
                        })
                    })
                    .collect()
            })
            .await
    }
}

fn filter_coastal(world: &World, mut positions: HashSet<V2<usize>>) -> HashSet<V2<usize>> {
    positions
        .retain(|position| world.get_cell_unsafe(&position).visible && is_coastal(world, position));
    positions
}

fn homeland_positions(settlements: &HashMap<V2<usize>, Settlement>) -> HashSet<V2<usize>> {
    settlements
        .values()
        .filter(|settlement| settlement.class == SettlementClass::Homeland)
        .map(|homeland| homeland.position)
        .collect()
}

fn get_positions_to_reveal(world: &World, from: &V2<usize>, to: &V2<usize>) -> HashSet<V2<usize>> {
    extend_all(
        world,
        unwrap_or!(get_voyage(world, from, to), return hashset! {}),
    )
}

fn get_voyage(world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Vec<V2<usize>>> {
    if !world.get_cell_unsafe(&to).visible {
        return None;
    }
    if !is_coastal(world, to) {
        return None;
    }
    let mut voyage: Vec<V2<usize>> =
        WalkGrid::new((from.x as i32, from.y as i32), (to.x as i32, to.y as i32))
            .map(|(x, y)| v2(x as usize, y as usize))
            .collect();
    if !voyage
        .iter()
        .all(|position| world.is_sea(position) || !world.get_cell_unsafe(position).visible)
    {
        return None;
    }
    if !voyage
        .iter()
        .any(|position| !world.get_cell_unsafe(position).visible)
    {
        return None;
    }
    Some(
        voyage
            .drain(..)
            .take_while(|position| world.is_sea(&position))
            .collect(),
    )
}

fn is_coastal(world: &World, position: &V2<usize>) -> bool {
    if !world.is_sea(position) {
        return false;
    }
    if !world.get_cell_unsafe(position).visible {
        return false;
    }
    world
        .neighbours(position)
        .iter()
        .any(|position| !world.is_sea(position) && world.get_cell_unsafe(position).visible)
}

fn extend_all(world: &World, positions: Vec<V2<usize>>) -> HashSet<V2<usize>> {
    positions
        .into_iter()
        .flat_map(|position| world.expand_position(&position))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::M;

    #[rustfmt::skip]
    fn world() -> World {

        World::new(
            M::from_vec(5, 5, vec![
                0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 1.0, 0.0,
                0.0, 1.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
            0.5
        )
        
    }

    #[test]
    fn test_is_coastal() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        assert!(is_coastal(&world, &v2(1, 0)));
    }

    #[test]
    fn test_not_coastal_if_land_invisible() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        assert!(!is_coastal(&world, &v2(1, 0)));
    }

    #[test]
    fn test_not_coastal_if_sea_invisible() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        assert!(!is_coastal(&world, &v2(1, 0)));
    }

    #[test]
    fn test_non_coast_sea_is_not_coastal() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(0, 0)).visible = true;
        assert!(!is_coastal(&world, &v2(0, 0)));
    }

    #[test]
    fn test_land_is_not_coastal() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        assert!(!is_coastal(&world, &v2(1, 1)));
    }

    #[test]
    fn test_get_voyage() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        world.mut_cell_unsafe(&v2(2, 1)).visible = true;
        assert_eq!(
            get_voyage(&world, &v2(4, 1), &v2(2, 1)),
            Some(vec![v2(4, 1), v2(3, 1), v2(2, 1)])
        );
    }

    #[test]
    fn test_from_visible_is_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        world.mut_cell_unsafe(&v2(2, 1)).visible = true;
        world.mut_cell_unsafe(&v2(4, 1)).visible = true;
        assert_eq!(
            get_voyage(&world, &v2(4, 1), &v2(2, 1)),
            Some(vec![v2(4, 1), v2(3, 1), v2(2, 1)])
        );
    }

    #[test]
    fn test_to_invisibile_not_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(4, 1)).visible = true;
        world.mut_cell_unsafe(&v2(3, 1)).visible = true;
        assert_eq!(get_voyage(&world, &v2(4, 1), &v2(2, 1)), None);
    }

    #[test]
    fn test_all_visibile_not_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        world.mut_cell_unsafe(&v2(2, 1)).visible = true;
        world.mut_cell_unsafe(&v2(3, 1)).visible = true;
        world.mut_cell_unsafe(&v2(4, 1)).visible = true;
        assert_eq!(get_voyage(&world, &v2(4, 1), &v2(2, 1)), None);
    }

    #[test]
    fn test_visible_land_in_way_not_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        assert_eq!(get_voyage(&world, &v2(1, 4), &v2(1, 0)), None)
    }

    #[test]
    fn test_invisible_land_in_way_is_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).visible = true;
        world.mut_cell_unsafe(&v2(2, 2)).visible = true;
        assert_eq!(
            get_voyage(&world, &v2(4, 2), &v2(2, 2)),
            Some(vec![v2(4, 2)])
        );
    }

    #[test]
    fn test_to_not_coastal_not_voyagle() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(2, 4)).visible = true;
        assert_eq!(get_voyage(&world, &v2(4, 4), &v2(2, 4)), None)
    }
}
