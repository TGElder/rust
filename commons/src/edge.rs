use std::error;
use std::fmt;

use crate::V2;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    from: V2<usize>,
    to: V2<usize>,
}

impl Edge {
    pub fn new(from: V2<usize>, to: V2<usize>) -> Edge {
        Self::new_safe(from, to).unwrap()
    }

    pub fn new_safe(from: V2<usize>, to: V2<usize>) -> Result<Edge, DiagonalEdge> {
        if to.x != from.x && to.y != from.y {
            return Err(DiagonalEdge { from, to });
        }
        if to.x > from.x || to.y > from.y {
            Ok(Edge { from, to })
        } else {
            Ok(Edge { from: to, to: from })
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

    pub fn length(&self) -> usize {
        (self.to.x - self.from.x) + (self.to.y - self.from.y)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct DiagonalEdge {
    pub from: V2<usize>,
    pub to: V2<usize>,
}

impl fmt::Display for DiagonalEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Diagonal edge {:?} from {:?}!", self.from, self.to)
    }
}

impl error::Error for DiagonalEdge {}

pub trait Edges {
    fn edges<'a>(&'a self) -> Box<dyn Iterator<Item = Edge> + 'a>;
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
    fn test_length() {
        let edge = Edge::new(v2(1, 10), v2(10, 10));
        assert_eq!(edge.length(), 9);

        let edge = Edge::new(v2(10, 1), v2(10, 10));
        assert_eq!(edge.length(), 9);
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
}
