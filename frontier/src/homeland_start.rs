use crate::world::World;
use commons::grid::Grid;
use commons::manhattan::ManhattanDistance;
use commons::rand::prelude::*;
use commons::{v2, V2};
use line_drawing::WalkGrid;

pub struct HomelandStart {
    pub homeland: V2<usize>,
    pub pre_landfall: V2<usize>,
    pub landfall: V2<usize>,
    pub voyage: Vec<V2<usize>>,
}

pub fn random_homeland_start<R: Rng>(world: &World, rng: &mut R) -> HomelandStart {
    let homeland = random_homeland_position(world, rng);
    let landfall = closest_position(&homeland, &land_positions(world));
    let voyage = voyage(&homeland, &landfall);
    let pre_landfall = *voyage.last().expect("Empty voyage");
    HomelandStart {
        homeland,
        pre_landfall,
        landfall,
        voyage,
    }
}

fn random_homeland_position<R: Rng>(world: &World, rng: &mut R) -> V2<usize> {
    *edge_positions(world)
        .choose(rng)
        .expect("No edge positions")
}

fn edge_positions(world: &World) -> Vec<V2<usize>> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            if x == 0 || x == world.width() - 1 || y == 0 || y == world.height() - 1 {
                out.push(v2(x, y));
            }
        }
    }
    out
}

fn land_positions(world: &World) -> Vec<V2<usize>> {
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

    #[test]
    #[rustfmt::skip]
    fn test_100_random_homeland_starts() {
        let world = World::new(
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
        );
        let mut rng = thread_rng();

        for _ in 0..100 {
            let start = random_homeland_start(&world, &mut rng);
            verify_homeland_start(&world, &start);
        }
    }

    fn verify_homeland_start(world: &World, start: &HomelandStart) {
        assert!(
            start.homeland.x == 0
                || start.homeland.x == 5
                || start.homeland.y == 0
                || start.homeland.y == 4
        );
        assert_no_closer_landfall(&world, &start.homeland, &start.landfall);
        assert!(!world.is_sea(&start.landfall));

        assert!(world.is_sea(&start.pre_landfall));

        assert_eq!(*start.voyage.first().unwrap(), start.homeland);
        assert_eq!(*start.voyage.last().unwrap(), start.pre_landfall);
        assert_eq!(
            start.voyage.len(),
            start.homeland.manhattan_distance(&start.pre_landfall) + 1
        );
        assert!(world.is_sea(start.voyage.last().unwrap()));
        assert_all_sea(&world, &start.voyage);
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
