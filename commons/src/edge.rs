use crate::{v2, V2};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    from: V2<usize>,
    to: V2<usize>,
}

impl Edge {
    pub fn new(from: V2<usize>, to: V2<usize>) -> Edge {
        if to.x > from.x && to.y > from.y {
            panic!("Diagonal edge {:?} from {:?}", from, to);
        }
        if to.x > from.x || to.y > from.y {
            Edge { from, to }
        } else {
            Edge { from: to, to: from }
        }
    }

    pub fn from(&self) -> &V2<usize> {
        &self.from
    }

    pub fn to(&self) -> &V2<usize> {
        &self.to
    }

    pub fn horizontal(&self) -> bool {
        self.from.y == self.to.y
    }

    pub fn unit_edges(&self) -> impl Iterator<Item = Edge> {
        let from_x = self.from.x;
        let to_x = self.to.x;
        let from_y = self.from.y;
        let to_y = self.to.y;

        let from = (from_x..=to_x).flat_map(move |x| (from_y..=to_y).map(move |y| v2(x, y)));
        let to = (from_x..=to_x)
            .flat_map(move |x| (from_y..=to_y).map(move |y| v2(x, y)))
            .skip(1);

        from.zip(to).map(|(from, to)| Edge::new(from, to))
    }
}

pub trait Edges {
    fn edges<'a>(&'a self) -> Box<dyn Iterator<Item = Edge> + 'a>;

    fn unit_edges<'a>(&'a self) -> Box<dyn Iterator<Item = Edge> + 'a> {
        Box::new(self.edges().flat_map(|edge| edge.unit_edges()))
    }
}

impl Edges for [V2<usize>] {
    fn edges<'a>(&'a self) -> Box<dyn Iterator<Item = Edge> + 'a> {
        if self.len() <= 1 {
            Box::new(std::iter::empty())
        } else {
            Box::new((0..self.len() - 1).map(move |i| Edge::new(self[i], self[i + 1])))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::v2;

    #[test]
    fn edges_should_be_canonical() {
        let edge = Edge::new(v2(1, 10), v2(10, 10));
        assert_eq!(
            edge,
            Edge {
                from: v2(1, 10),
                to: v2(10, 10)
            }
        );

        let edge = Edge::new(v2(10, 10), v2(1, 10));
        assert_eq!(
            edge,
            Edge {
                from: v2(1, 10),
                to: v2(10, 10)
            }
        );

        let edge = Edge::new(v2(10, 1), v2(10, 10));
        assert_eq!(
            edge,
            Edge {
                from: v2(10, 1),
                to: v2(10, 10)
            }
        );

        let edge = Edge::new(v2(10, 10), v2(10, 1));
        assert_eq!(
            edge,
            Edge {
                from: v2(10, 1),
                to: v2(10, 10)
            }
        );
    }

    #[test]
    fn test_horizontal() {
        let edge = Edge::new(v2(1, 10), v2(10, 10));
        assert!(edge.horizontal());

        let edge = Edge::new(v2(10, 1), v2(10, 10));
        assert!(!edge.horizontal());
    }

    #[test]
    fn test_unit_edges_horizontal() {
        let edge = Edge::new(v2(1, 1), v2(3, 1));
        assert_eq!(
            edge.unit_edges().collect::<Vec<_>>(),
            vec![Edge::new(v2(1, 1), v2(2, 1)), Edge::new(v2(2, 1), v2(3, 1))]
        );
    }

    #[test]
    fn test_unit_edges_vertical() {
        let edge = Edge::new(v2(1, 1), v2(1, 3));
        assert_eq!(
            edge.unit_edges().collect::<Vec<_>>(),
            vec![Edge::new(v2(1, 1), v2(1, 2)), Edge::new(v2(1, 2), v2(1, 3))]
        );
    }

    #[test]
    fn test_edges_with_vector() {
        let positions = vec![v2(0, 0), v2(1, 0), v2(2, 0)];
        let edges: Vec<Edge> = positions.edges().collect();
        assert_eq!(
            edges,
            vec![Edge::new(v2(0, 0), v2(1, 0)), Edge::new(v2(1, 0), v2(2, 0))]
        );
    }

    #[test]
    fn test_edges_with_array() {
        let positions = [v2(0, 0), v2(1, 0), v2(2, 0)];
        let edges: Vec<Edge> = positions.edges().collect();
        assert_eq!(
            edges,
            vec![Edge::new(v2(0, 0), v2(1, 0)), Edge::new(v2(1, 0), v2(2, 0))]
        );
    }

    #[test]
    fn test_edges_singleton_list() {
        let positions = vec![v2(0, 0)];
        let edges = positions.edges();
        assert_eq!(edges.count(), 0);
    }

    #[test]
    fn test_edges_empty_list() {
        let positions = vec![];
        let edges = positions.edges();
        assert_eq!(edges.count(), 0);
    }

    #[test]
    fn test_unit_edges_with_vector() {
        let positions = vec![v2(0, 0), v2(2, 0), v2(3, 0), v2(3, 2)];
        let edges: Vec<Edge> = positions.unit_edges().collect();
        assert_eq!(
            edges,
            vec![
                Edge::new(v2(0, 0), v2(1, 0)),
                Edge::new(v2(1, 0), v2(2, 0)),
                Edge::new(v2(2, 0), v2(3, 0)),
                Edge::new(v2(3, 0), v2(3, 1)),
                Edge::new(v2(3, 1), v2(3, 2))
            ]
        );
    }
}
