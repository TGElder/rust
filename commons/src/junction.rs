use crate::edge::*;
use crate::{v2, V2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Junction1D {
    pub width: f32,
    pub from: bool,
    pub to: bool,
}

#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Junction {
    pub horizontal: Junction1D,
    pub vertical: Junction1D,
}

impl Junction {
    pub fn width(&self) -> f32 {
        self.vertical.width
    }

    pub fn height(&self) -> f32 {
        self.horizontal.width
    }

    pub fn longest_side(&self) -> f32 {
        self.width().max(self.height())
    }

    pub fn here(&self) -> bool {
        self.width() > 0.0 || self.height() > 0.0
    }

    pub fn corner(&self) -> bool {
        self.width() > 0.0 && self.height() > 0.0
    }

    pub fn junction_1d(&mut self, horizontal: bool) -> &mut Junction1D {
        if horizontal {
            &mut self.horizontal
        } else {
            &mut self.vertical
        }
    }

    fn get_horizontal_edge_from(&self, position: &V2<usize>) -> Option<Edge> {
        if self.horizontal.from {
            Some(Edge::new(*position, position + v2(1, 0)))
        } else {
            None
        }
    }

    fn get_vertical_edge_from(&self, position: &V2<usize>) -> Option<Edge> {
        if self.vertical.from {
            Some(Edge::new(*position, position + v2(0, 1)))
        } else {
            None
        }
    }

    pub fn get_edges_from(&self, position: &V2<usize>) -> Vec<Edge> {
        vec![
            self.get_horizontal_edge_from(position),
            self.get_vertical_edge_from(position),
        ]
        .iter()
        .flatten()
        .cloned()
        .collect()
    }
}

#[cfg(test)]
mod tests {

    use crate::almost::Almost;

    use super::*;

    #[test]
    fn longest_side() {
        let junction = Junction {
            horizontal: Junction1D {
                width: 0.2,
                ..Junction1D::default()
            },
            vertical: Junction1D {
                width: 0.3,
                ..Junction1D::default()
            },
        };
        assert!(junction.longest_side().almost(&0.3));

        let junction = Junction {
            horizontal: Junction1D {
                width: 0.4,
                ..Junction1D::default()
            },
            vertical: Junction1D {
                width: 0.1,
                ..Junction1D::default()
            },
        };
        assert!(junction.longest_side().almost(&0.4));
    }

    #[test]
    fn get_horizontal_edge_from_false() {
        let junction = Junction::default();
        let actual = junction.get_horizontal_edge_from(&v2(1, 1));
        assert_eq!(actual, None);
    }

    #[test]
    fn get_horizontal_edge_from() {
        let mut junction = Junction::default();
        junction.horizontal.from = true;
        let actual = junction.get_horizontal_edge_from(&v2(1, 1));
        assert_eq!(actual, Some(Edge::new(v2(1, 1), v2(2, 1))));
    }

    #[test]
    fn get_vertical_edge_from_false() {
        let junction = Junction::default();
        let actual = junction.get_vertical_edge_from(&v2(1, 1));
        assert_eq!(actual, None);
    }

    #[test]
    fn get_vertical_edge_from() {
        let mut junction = Junction::default();
        junction.vertical.from = true;
        let actual = junction.get_vertical_edge_from(&v2(1, 1));
        assert_eq!(actual, Some(Edge::new(v2(1, 1), v2(1, 2))));
    }

    #[test]
    fn get_edges_from() {
        let mut junction = Junction::default();
        assert_eq!(junction.get_edges_from(&v2(1, 1)), vec![]);
        junction.horizontal.from = true;
        assert_eq!(
            junction.get_edges_from(&v2(1, 1)),
            vec![Edge::new(v2(1, 1), v2(2, 1))]
        );
        junction.horizontal.from = false;
        junction.vertical.from = true;
        assert_eq!(
            junction.get_edges_from(&v2(1, 1)),
            vec![Edge::new(v2(1, 1), v2(1, 2))]
        );
        junction.horizontal.from = true;
        assert_eq!(
            junction.get_edges_from(&v2(1, 1)),
            vec![Edge::new(v2(1, 1), v2(2, 1)), Edge::new(v2(1, 1), v2(1, 2))]
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionJunction {
    pub position: V2<usize>,
    pub junction: Junction,
}

impl PositionJunction {
    pub fn new(position: V2<usize>) -> PositionJunction {
        PositionJunction {
            position,
            junction: Junction::default(),
        }
    }
}
