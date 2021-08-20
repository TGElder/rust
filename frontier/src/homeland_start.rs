use crate::world::World;
use commons::grid::Grid;
use commons::manhattan::ManhattanDistance;
use commons::rand::prelude::*;
use commons::{v2, V2};
use line_drawing::WalkGrid;
use serde::{Deserialize, Serialize};

pub struct HomelandStartGen<'a, R>
where
    R: Rng,
{
    world: &'a World,
    rng: &'a mut R,
    edges: &'a [HomelandEdge],
    min_distance_between_homelands: Option<usize>,
    existing_homelands: Vec<V2<usize>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HomelandEdge {
    North,
    South,
    East,
    West,
}

impl HomelandEdge {
    fn position_is_permitted(&self, world: &World, position: &V2<usize>) -> bool {
        match self {
            HomelandEdge::North => position.y == 0,
            HomelandEdge::East => position.x == world.width() - 1,
            HomelandEdge::South => position.y == world.height() - 1,
            HomelandEdge::West => position.x == 0,
        }
    }
}

#[derive(Clone)]
pub struct HomelandStart {
    pub homeland: V2<usize>,
    pub pre_landfall: V2<usize>,
    pub landfall: V2<usize>,
    pub voyage: Vec<V2<usize>>,
}

impl<'a, R> HomelandStartGen<'a, R>
where
    R: Rng,
{
    pub fn new(
        world: &'a World,
        rng: &'a mut R,
        edges: &'a [HomelandEdge],
        min_distance_between_homelands: Option<usize>,
    ) -> HomelandStartGen<'a, R> {
        HomelandStartGen {
            world,
            rng,
            edges,
            min_distance_between_homelands,
            existing_homelands: vec![],
        }
    }

    pub fn random_start(&mut self) -> HomelandStart {
        let homeland = self.random_homeland_position();
        self.existing_homelands.push(homeland);
        let landfall = closest_position(&homeland, &self.land_positions());
        let voyage = voyage(&homeland, &landfall);
        let pre_landfall = *voyage.last().expect("Empty voyage");
        HomelandStart {
            homeland,
            pre_landfall,
            landfall,
            voyage,
        }
    }

    fn random_homeland_position(&mut self) -> V2<usize> {
        *self
            .edge_positions()
            .choose(self.rng)
            .expect("No edge positions")
    }

    fn edge_positions(&self) -> Vec<V2<usize>> {
        let world = self.world;
        let mut out = vec![];
        for x in 0..world.width() {
            for y in 0..world.height() {
                let position = v2(x, y);
                if self.position_on_permitted_edge(&position)
                    && !self.too_close_to_existing_homeland(&position)
                {
                    out.push(position);
                }
            }
        }
        out
    }

    fn too_close_to_existing_homeland(&self, position: &V2<usize>) -> bool {
        let min_distance = unwrap_or!(self.min_distance_between_homelands, return false);
        self.existing_homelands
            .iter()
            .any(|homeland| homeland.manhattan_distance(position) < min_distance)
    }

    fn position_on_permitted_edge(&self, position: &V2<usize>) -> bool {
        self.edges
            .iter()
            .any(|edge| edge.position_is_permitted(self.world, position))
    }

    fn land_positions(&self) -> Vec<V2<usize>> {
        let world = self.world;
        let mut out = vec![];
        for x in 0..world.width() {
            for y in 0..world.height() {
                let position = v2(x, y);
                if !world.is_sea(&position) {
                    out.push(position);
                }
            }
        }
        out
    }
}

fn closest_position(position: &V2<usize>, others: &[V2<usize>]) -> V2<usize> {
    *others
        .iter()
        .min_by(|a, b| {
            a.manhattan_distance(position)
                .cmp(&b.manhattan_distance(position))
        })
        .expect("No land positions")
}

fn voyage(from: &V2<usize>, to: &V2<usize>) -> Vec<V2<usize>> {
    let mut out: Vec<V2<usize>> =
        WalkGrid::new((from.x as i32, from.y as i32), (to.x as i32, to.y as i32))
            .map(|(x, y): (i32, i32)| v2(x as usize, y as usize))
            .collect();
    out.pop(); // Don't want the land position
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::M;

    #[rustfmt::skip]
    fn world() -> World {
        World::new(
            M::from_vec(
                6,
                5,
                vec![
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                    0.0, 0.0, 1.0, 1.0, 0.0, 0.0,
                    0.0, 0.0, 0.0, 1.0, 1.0, 0.0,
                    0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                ],
            ),
            0.0,
        )
    }

    #[test]
    fn test_100_random_starts() {
        let world = world();
        let mut rng = thread_rng();

        let mut gen = HomelandStartGen::new(
            &world,
            &mut rng,
            &[
                HomelandEdge::North,
                HomelandEdge::East,
                HomelandEdge::South,
                HomelandEdge::West,
            ],
            None,
        );

        for _ in 0..100 {
            let start = gen.random_start();
            verify_homeland_start(&world, &start);
        }
    }

    #[test]
    fn test_100_random_starts_north_edge_only() {
        let world = world();
        let mut rng = thread_rng();

        let mut gen = HomelandStartGen::new(&world, &mut rng, &[HomelandEdge::North], None);

        for _ in 0..100 {
            let start = gen.random_start();
            assert_eq!(start.homeland.y, 0);
        }
    }

    #[test]
    fn test_100_random_starts_east_edge_only() {
        let world = world();
        let mut rng = thread_rng();

        let mut gen = HomelandStartGen::new(&world, &mut rng, &[HomelandEdge::East], None);

        for _ in 0..100 {
            let start = gen.random_start();
            assert_eq!(start.homeland.x, 5);
        }
    }

    #[test]
    fn test_100_random_starts_south_edge_only() {
        let world = world();
        let mut rng = thread_rng();

        let mut gen = HomelandStartGen::new(&world, &mut rng, &[HomelandEdge::South], None);

        for _ in 0..100 {
            let start = gen.random_start();
            assert_eq!(start.homeland.y, 4);
        }
    }

    #[test]
    fn test_100_random_starts_west_edge_only() {
        let world = world();
        let mut rng = thread_rng();

        let mut gen = HomelandStartGen::new(&world, &mut rng, &[HomelandEdge::West], None);

        for _ in 0..100 {
            let start = gen.random_start();
            assert_eq!(start.homeland.x, 0);
        }
    }

    #[test]
    fn test_100_random_pairs_min_distance() {
        let world = world();
        let mut rng = thread_rng();

        for _ in 0..100 {
            let mut gen = HomelandStartGen::new(
                &world,
                &mut rng,
                &[
                    HomelandEdge::North,
                    HomelandEdge::East,
                    HomelandEdge::South,
                    HomelandEdge::West,
                ],
                Some(3),
            );
            let start_1 = gen.random_start();
            let start_2 = gen.random_start();

            assert!(start_1.homeland.manhattan_distance(&start_2.homeland) >= 3);
        }
    }

    fn verify_homeland_start(world: &World, start: &HomelandStart) {
        assert!(
            start.homeland.x == 0
                || start.homeland.x == 5
                || start.homeland.y == 0
                || start.homeland.y == 4
        );
        assert_no_closer_landfall(world, &start.homeland, &start.landfall);
        assert!(!world.is_sea(&start.landfall));

        assert!(world.is_sea(&start.pre_landfall));

        assert_eq!(*start.voyage.first().unwrap(), start.homeland);
        assert_eq!(*start.voyage.last().unwrap(), start.pre_landfall);
        assert_eq!(
            start.voyage.len(),
            start.homeland.manhattan_distance(&start.pre_landfall) + 1
        );
        assert!(world.is_sea(start.voyage.last().unwrap()));
        assert_all_sea(world, &start.voyage);
    }

    fn assert_no_closer_landfall(world: &World, from: &V2<usize>, to: &V2<usize>) {
        let distance_to_beat = from.manhattan_distance(to);
        for x in 0..world.width() {
            for y in 0..world.height() {
                let position = v2(x, y);
                if position == *to {
                    continue;
                }
                if !world.is_sea(&position) {
                    assert!(from.manhattan_distance(&position) >= distance_to_beat);
                }
            }
        }
    }

    fn assert_all_sea(world: &World, voyage: &[V2<usize>]) -> bool {
        voyage.iter().all(|position| world.is_sea(position))
    }
}
