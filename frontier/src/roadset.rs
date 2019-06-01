use commons::*;
use isometric::terrain::*;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
struct HalfJunction {
    width: f32,
    from: bool,
    to: bool,
}

impl HalfJunction {
    fn new(width: f32) -> HalfJunction {
        HalfJunction {
            width,
            from: false,
            to: false,
        }
    }

    fn any(&self) -> bool {
        self.from || self.to
    }

    fn width(&self) -> f32 {
        if self.any() {
            self.width
        } else {
            0.0
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
struct Junction {
    horizontal: HalfJunction,
    vertical: HalfJunction,
}

impl Junction {
    fn new(width: f32) -> Junction {
        Junction {
            horizontal: HalfJunction::new(width),
            vertical: HalfJunction::new(width),
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct RoadSet {
    junctions: M<Junction>,
}

impl RoadSet {
    pub fn new(width: usize, height: usize, road_width: f32) -> RoadSet {
        RoadSet {
            junctions: M::from_element(width, height, Junction::new(road_width)),
        }
    }

    fn get_junction(&self, position: &V2<usize>) -> &Junction {
        &self.junctions[(position.x, position.y)]
    }

    fn get_junction_mut(&mut self, position: &V2<usize>) -> &mut Junction {
        &mut self.junctions[(position.x, position.y)]
    }

    pub fn set_widths_from_nodes(&mut self, nodes: &Vec<Node>) {
        for node in nodes {
            let mut junction = self.get_junction_mut(&node.position());
            if node.width() > 0.0 {
                junction.vertical.width = node.width();
            }
            if node.height() > 0.0 {
                junction.horizontal.width = node.height();
            }
        }
    }

    pub fn add_road(&mut self, road: &Edge) {
        let mut from_junction = self.get_junction_mut(road.from());
        if road.horizontal() {
            from_junction.horizontal.from = true;
        } else {
            from_junction.vertical.from = true;
        }
        let mut to_junction = self.get_junction_mut(road.to());
        if road.horizontal() {
            to_junction.horizontal.to = true;
        } else {
            to_junction.vertical.to = true;
        }
    }

    pub fn add_roads(&mut self, edges: &Vec<Edge>) {
        for edge in edges.iter() {
            self.add_road(edge);
        }
    }

    pub fn clear_road(&mut self, road: &Edge) {
        let mut from_junction = self.get_junction_mut(road.from());
        if road.horizontal() {
            from_junction.horizontal.from = false;
        } else {
            from_junction.vertical.from = false;
        }
        let mut to_junction = self.get_junction_mut(road.to());
        if road.horizontal() {
            to_junction.horizontal.to = false;
        } else {
            to_junction.vertical.to = false;
        }
    }

    pub fn get_horizontal_width(&self, position: &V2<usize>) -> f32 {
        self.get_junction(position).horizontal.width()
    }

    pub fn get_vertical_width(&self, position: &V2<usize>) -> f32 {
        self.get_junction(position).vertical.width()
    }

    pub fn along(&self, edge: &Edge) -> bool {
        if edge.horizontal() {
            self.get_junction(&edge.from()).horizontal.from
        } else {
            self.get_junction(&edge.from()).vertical.from
        }
    }

    pub fn get_node(&self, position: V2<usize>) -> Node {
        let width = self.get_vertical_width(&position);
        let height = self.get_horizontal_width(&position);
        Node::new(position, width, height)
    }

    pub fn get_nodes(&self, from: &V2<usize>, to: &V2<usize>) -> Vec<Node> {
        let mut out = vec![];
        for x in from.x..to.x {
            for y in from.y..to.y {
                let node = self.get_node(v2(x, y));
                if node.width() > 0.0 || node.height() > 0.0 {
                    out.push(node);
                }
            }
        }
        out
    }

    pub fn get_edges(&self, from: &V2<usize>, to: &V2<usize>) -> Vec<Edge> {
        let mut out = vec![];
        for x in from.x..to.x {
            for y in from.y..to.y {
                let from = v2(x, y);
                let junction = self.get_junction(&from);
                if junction.horizontal.from {
                    out.push(Edge::new(from, v2(x + 1, y)));
                }
                if junction.vertical.from {
                    out.push(Edge::new(from, v2(x, y + 1)));
                }
            }
        }
        out
    }

    pub fn width_here(&self, position: &V2<usize>) -> f32 {
        self.get_horizontal_width(position)
            .max(self.get_vertical_width(position))
    }

    pub fn here(&self, position: &V2<usize>) -> bool {
        self.width_here(position) > 0.0
    }

    pub fn corner_here(&self, position: &V2<usize>) -> bool {
        self.get_horizontal_width(position) > 0.0 && self.get_vertical_width(position) > 0.0
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn l() -> RoadSet {
        let mut roadset = RoadSet::new(2, 2, 9.0);
        roadset.add_road(&Edge::new(v2(0, 0), v2(1, 0)));
        roadset.add_road(&Edge::new(v2(0, 0), v2(0, 1)));
        roadset
    }

    fn parallel() -> RoadSet {
        let mut roadset = RoadSet::new(2, 2, 9.0);
        roadset.add_road(&Edge::new(v2(0, 0), v2(1, 0)));
        roadset.add_road(&Edge::new(v2(0, 1), v2(1, 1)));
        roadset
    }

    #[test]
    fn test_set_widths_from_nodes() {
        let mut roadset = l();
        roadset.set_widths_from_nodes(&vec![
            Node::new(v2(0, 0), 0.1, 0.0),
            Node::new(v2(0, 0), 0.0, 0.2),
            Node::new(v2(1, 0), 0.3, 0.0),
            Node::new(v2(1, 0), 0.0, 0.4),
            Node::new(v2(0, 1), 0.5, 0.6),
            Node::new(v2(1, 1), 0.7, 0.8),
        ]);
        assert_eq!(
            roadset.get_junction(&v2(0, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 0.2,
                    from: true,
                    to: false
                },
                vertical: HalfJunction {
                    width: 0.1,
                    from: true,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(1, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 0.4,
                    from: false,
                    to: true
                },
                vertical: HalfJunction {
                    width: 0.3,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(0, 1)),
            &Junction {
                horizontal: HalfJunction {
                    width: 0.6,
                    from: false,
                    to: false
                },
                vertical: HalfJunction {
                    width: 0.5,
                    from: false,
                    to: true
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(1, 1)),
            &Junction {
                horizontal: HalfJunction {
                    width: 0.8,
                    from: false,
                    to: false
                },
                vertical: HalfJunction {
                    width: 0.7,
                    from: false,
                    to: false
                }
            }
        );
    }

    #[test]
    fn test_add_road_l() {
        let roadset = l();
        assert_eq!(
            roadset.get_junction(&v2(0, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: true,
                    to: false
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: true,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(1, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: true
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(0, 1)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: true
                }
            }
        );
        assert_eq!(roadset.get_junction(&v2(1, 1)), &Junction::new(9.0));
    }

    #[test]
    fn test_add_road_parallel() {
        let roadset = parallel();
        assert_eq!(
            roadset.get_junction(&v2(0, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: true,
                    to: false
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(1, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: true
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(0, 1)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: true,
                    to: false
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(1, 1)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: true
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
    }

    #[test]
    fn test_add_roads() {
        let mut roadset = RoadSet::new(2, 2, 9.0);
        roadset.add_roads(&vec![
            Edge::new(v2(0, 0), v2(1, 0)),
            Edge::new(v2(0, 0), v2(0, 1)),
        ]);
        assert_eq!(
            roadset.get_junction(&v2(0, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: true,
                    to: false
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: true,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(1, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: true
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(0, 1)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: true
                }
            }
        );
        assert_eq!(roadset.get_junction(&v2(1, 1)), &Junction::new(9.0));
    }

    #[test]
    fn test_clear_road_l() {
        let mut roadset = l();
        roadset.clear_road(&Edge::new(v2(0, 0), v2(0, 1)));
        assert_eq!(
            roadset.get_junction(&v2(0, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: true,
                    to: false
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(1, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: true
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(roadset.get_junction(&v2(0, 1)), &Junction::new(9.0));
        assert_eq!(roadset.get_junction(&v2(1, 1)), &Junction::new(9.0));
    }

    #[test]
    fn test_clear_road_parallel() {
        let mut roadset = parallel();
        roadset.clear_road(&Edge::new(v2(0, 1), v2(1, 1)));
        assert_eq!(
            roadset.get_junction(&v2(0, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: true,
                    to: false
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(
            roadset.get_junction(&v2(1, 0)),
            &Junction {
                horizontal: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: true
                },
                vertical: HalfJunction {
                    width: 9.0,
                    from: false,
                    to: false
                }
            }
        );
        assert_eq!(roadset.get_junction(&v2(0, 1)), &Junction::new(9.0));
        assert_eq!(roadset.get_junction(&v2(1, 1)), &Junction::new(9.0));
    }

    #[test]
    fn test_get_horizontal_width_l() {
        let roadset = l();
        assert_eq!(roadset.get_horizontal_width(&v2(0, 0)), 9.0);
        assert_eq!(roadset.get_horizontal_width(&v2(1, 0)), 9.0);
        assert_eq!(roadset.get_horizontal_width(&v2(0, 1)), 0.0);
        assert_eq!(roadset.get_horizontal_width(&v2(1, 1)), 0.0);
    }

    #[test]
    fn test_get_vertical_width_l() {
        let roadset = l();
        assert_eq!(roadset.get_vertical_width(&v2(0, 0)), 9.0);
        assert_eq!(roadset.get_vertical_width(&v2(1, 0)), 0.0);
        assert_eq!(roadset.get_vertical_width(&v2(0, 1)), 9.0);
        assert_eq!(roadset.get_vertical_width(&v2(1, 1)), 0.0);
    }

    #[test]
    fn test_along_l() {
        let roadset = l();
        assert!(roadset.along(&Edge::new(v2(0, 0), v2(1, 0))));
        assert!(roadset.along(&Edge::new(v2(0, 0), v2(0, 1))));
        assert!(!roadset.along(&Edge::new(v2(0, 1), v2(1, 1))));
        assert!(!roadset.along(&Edge::new(v2(1, 0), v2(1, 1))));
    }

    #[test]
    fn test_get_horizontal_width_parallel() {
        let roadset = parallel();
        assert_eq!(roadset.get_horizontal_width(&v2(0, 0)), 9.0);
        assert_eq!(roadset.get_horizontal_width(&v2(1, 0)), 9.0);
        assert_eq!(roadset.get_horizontal_width(&v2(0, 1)), 9.0);
        assert_eq!(roadset.get_horizontal_width(&v2(1, 1)), 9.0);
    }

    #[test]
    fn test_get_vertical_width_parallel() {
        let roadset = parallel();
        assert_eq!(roadset.get_vertical_width(&v2(0, 0)), 0.0);
        assert_eq!(roadset.get_vertical_width(&v2(1, 0)), 0.0);
        assert_eq!(roadset.get_vertical_width(&v2(0, 1)), 0.0);
        assert_eq!(roadset.get_vertical_width(&v2(1, 1)), 0.0);
    }

    #[test]
    fn test_along_parallel() {
        let roadset = parallel();
        assert!(roadset.along(&Edge::new(v2(0, 0), v2(1, 0))));
        assert!(roadset.along(&Edge::new(v2(0, 1), v2(1, 1))));
        assert!(!roadset.along(&Edge::new(v2(0, 0), v2(0, 1))));
        assert!(!roadset.along(&Edge::new(v2(1, 0), v2(1, 1))));
    }

    #[test]
    fn test_get_nodes_l() {
        let roadset = l();
        let actual = roadset.get_nodes(&v2(0, 0), &v2(2, 2));
        assert_eq!(actual.len(), 3);
        assert!(actual.contains(&Node::new(v2(0, 0), 9.0, 9.0)));
        assert!(actual.contains(&Node::new(v2(1, 0), 0.0, 9.0)));
        assert!(actual.contains(&Node::new(v2(0, 1), 9.0, 0.0)));
    }

    #[test]
    fn test_get_nodes_parallel() {
        let roadset = parallel();
        let actual = roadset.get_nodes(&v2(0, 0), &v2(2, 2));
        assert_eq!(actual.len(), 4);
        assert!(actual.contains(&Node::new(v2(0, 0), 0.0, 9.0)));
        assert!(actual.contains(&Node::new(v2(1, 0), 0.0, 9.0)));
        assert!(actual.contains(&Node::new(v2(0, 1), 0.0, 9.0)));
        assert!(actual.contains(&Node::new(v2(1, 1), 0.0, 9.0)));
    }

    #[test]
    fn test_get_nodes_partial() {
        let roadset = l();
        let actual = roadset.get_nodes(&v2(0, 0), &v2(1, 1));
        assert_eq!(actual.len(), 1);
        assert!(actual.contains(&Node::new(v2(0, 0), 9.0, 9.0)));
    }

    #[test]
    fn test_get_edges_l() {
        let roadset = l();
        let actual = roadset.get_edges(&v2(0, 0), &v2(2, 2));
        assert_eq!(actual.len(), 2);
        assert!(actual.contains(&Edge::new(v2(0, 0), v2(1, 0))));
        assert!(actual.contains(&Edge::new(v2(0, 0), v2(0, 1))));
    }

    #[test]
    fn test_get_edges_parallel() {
        let roadset = parallel();
        let actual = roadset.get_edges(&v2(0, 0), &v2(2, 2));
        assert_eq!(actual.len(), 2);
        assert!(actual.contains(&Edge::new(v2(0, 0), v2(1, 0))));
        assert!(actual.contains(&Edge::new(v2(0, 1), v2(1, 1))));
    }

    #[test]
    fn test_width_here_parallel() {
        let roadset = parallel();
        assert_eq!(roadset.width_here(&v2(0, 0)), 9.0);
        assert_eq!(roadset.width_here(&v2(1, 0)), 9.0);
        assert_eq!(roadset.width_here(&v2(0, 1)), 9.0);
        assert_eq!(roadset.width_here(&v2(1, 1)), 9.0);
    }

    #[test]
    fn test_width_here_l() {
        let roadset = l();
        assert_eq!(roadset.width_here(&v2(0, 0)), 9.0);
        assert_eq!(roadset.width_here(&v2(1, 0)), 9.0);
        assert_eq!(roadset.width_here(&v2(0, 1)), 9.0);
        assert_eq!(roadset.width_here(&v2(1, 1)), 0.0);
    }

    #[test]
    fn test_here_parallel() {
        let roadset = parallel();
        assert_eq!(roadset.here(&v2(0, 0)), true);
        assert_eq!(roadset.here(&v2(1, 0)), true);
        assert_eq!(roadset.here(&v2(0, 1)), true);
        assert_eq!(roadset.here(&v2(1, 1)), true);
    }

    #[test]
    fn test_here_l() {
        let roadset = l();
        assert_eq!(roadset.here(&v2(0, 0)), true);
        assert_eq!(roadset.here(&v2(1, 0)), true);
        assert_eq!(roadset.here(&v2(0, 1)), true);
        assert_eq!(roadset.here(&v2(1, 1)), false);
    }

    #[test]
    fn test_corner_here_parallel() {
        let roadset = parallel();
        assert_eq!(roadset.corner_here(&v2(0, 0)), false);
        assert_eq!(roadset.corner_here(&v2(1, 0)), false);
        assert_eq!(roadset.corner_here(&v2(0, 1)), false);
        assert_eq!(roadset.corner_here(&v2(1, 1)), false);
    }

    #[test]
    fn test_corner_here_l() {
        let roadset = l();
        assert_eq!(roadset.corner_here(&v2(0, 0)), true);
        assert_eq!(roadset.corner_here(&v2(1, 0)), false);
        assert_eq!(roadset.corner_here(&v2(0, 1)), false);
        assert_eq!(roadset.corner_here(&v2(1, 1)), false);
    }

    #[test]
    fn round_trip() {
        let original = l();
        let encoded: Vec<u8> = bincode::serialize(&original).unwrap();
        let reconstructed: RoadSet = bincode::deserialize(&encoded[..]).unwrap();
        assert_eq!(original, reconstructed);
    }

}
