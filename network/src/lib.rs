use std::cmp::Ordering;
use std::collections::BinaryHeap;
#[cfg(test)]
#[macro_use]
extern crate hamcrest;

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub cost: u8,
}

impl Edge {
    pub fn new(from: usize, to: usize, cost: u8) -> Edge {
        Edge { from, to, cost }
    }

    pub fn create_4_neighbour_deltas() -> Vec<(usize, usize)> {
        vec![(1, 0), (0, 1)]
    }

    pub fn create_8_neighbour_deltas() -> Vec<(usize, usize)> {
        vec![(1, 0), (1, 1), (0, 1)]
    }

    pub fn create_grid(
        width: usize,
        height: usize,
        cost: u8,
        neighbour_deltas: Vec<(usize, usize)>,
    ) -> Vec<Edge> {
        fn get_index(x: usize, y: usize, width: usize) -> usize {
            (y * width) + x
        }

        fn create_edge(
            x: usize,
            y: usize,
            width: usize,
            height: usize,
            delta: &(usize, usize),
            cost: u8,
        ) -> Vec<Edge> {
            let x_b = x + delta.0;
            let y_b = y + delta.1;
            if (x_b >= width) || (y_b >= height) {
                return vec![];
            }
            let index_a = get_index(x, y, width);
            let index_b = get_index(x_b, y_b, width);
            vec![
                Edge::new(index_a, index_b, cost),
                Edge::new(index_b, index_a, cost),
            ]
        }

        neighbour_deltas
            .iter()
            .flat_map(move |d| {
                (0..width).flat_map(move |x| {
                    (0..height).flat_map(move |y| create_edge(x, y, width, height, d, cost))
                })
            })
            .collect()
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct Network {
    pub nodes: usize,
    edges_out: Vec<Vec<Edge>>,
    edges_in: Vec<Vec<Edge>>,
}

impl Network {
    pub fn new(nodes: usize, edges: &[Edge]) -> Network {
        let mut out = Network {
            nodes,
            edges_out: vec![vec![]; nodes],
            edges_in: vec![vec![]; nodes],
        };

        edges.iter().for_each(|edge| out.add_edge(edge));

        out
    }

    pub fn add_edge(&mut self, edge: &Edge) {
        self.edges_out
            .get_mut(edge.from)
            .unwrap()
            .push(edge.clone());
        self.edges_in.get_mut(edge.to).unwrap().push(edge.clone());
    }

    pub fn remove_edges(&mut self, from: usize, to: usize) {
        self.edges_out[from]
            .iter()
            .position(|edge| edge.to == to)
            .map(|i| self.edges_out[from].remove(i));
        self.edges_in[to]
            .iter()
            .position(|edge| edge.from == from)
            .map(|i| self.edges_in[to].remove(i));
    }

    pub fn get_in(&self, node: usize) -> &Vec<Edge> {
        &self.edges_in[node]
    }

    pub fn get_out(&self, node: usize) -> &Vec<Edge> {
        &self.edges_out[node]
    }

    pub fn dijkstra(&self, nodes: Vec<usize>) -> Vec<Option<u32>> {
        #[derive(Eq)]
        struct Node {
            index: usize,
            cost: u32,
        }

        impl Ord for Node {
            fn cmp(&self, other: &Node) -> Ordering {
                self.cost.cmp(&other.cost).reverse()
            }
        }

        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Node) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl PartialEq for Node {
            fn eq(&self, other: &Node) -> bool {
                self.cost == other.cost
            }
        }

        let mut closed: Vec<bool> = vec![false; self.nodes];
        let mut out: Vec<Option<u32>> = vec![None; self.nodes];
        let mut heap = BinaryHeap::new();

        for node in nodes {
            heap.push(Node {
                index: node,
                cost: 0,
            });
        }

        while let Some(Node { index, cost }) = heap.pop() {
            if !closed[index] {
                closed[index] = true;
                out[index] = Some(cost);

                for edge in self.get_in(index) {
                    if !closed[edge.from] {
                        heap.push(Node {
                            index: edge.from,
                            cost: cost + u32::from(edge.cost),
                        });
                    }
                }
            }
        }

        out
    }

    pub fn find_path(
        &self,
        from: usize,
        to: usize,
        max_cost: Option<u32>,
        heuristic: &dyn Fn(usize) -> u32,
    ) -> Option<Vec<Edge>> {
        #[derive(Eq)]
        struct Node {
            index: usize,
            entry: Option<Edge>,
            distance_from_start: u32,
            estimated_path_distance_via_this_node: u32,
        }

        impl Node {
            fn new(
                index: usize,
                entry: Option<Edge>,
                distance_from_start: u32,
                heuristic: &dyn Fn(usize) -> u32,
            ) -> Node {
                let estimated_distance_to_goal = heuristic(index);
                Node {
                    index,
                    entry,
                    distance_from_start,
                    estimated_path_distance_via_this_node: distance_from_start
                        + estimated_distance_to_goal,
                }
            }
        }

        impl Ord for Node {
            fn cmp(&self, other: &Node) -> Ordering {
                self.estimated_path_distance_via_this_node
                    .cmp(&other.estimated_path_distance_via_this_node)
                    .reverse()
            }
        }

        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Node) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl PartialEq for Node {
            fn eq(&self, other: &Node) -> bool {
                self.estimated_path_distance_via_this_node
                    == other.estimated_path_distance_via_this_node
            }
        }

        fn get_path(from: usize, to: usize, edges: &[Option<Edge>]) -> Vec<Edge> {
            let mut out = vec![];
            let mut current = to;
            while current != from {
                if let Some(Some(edge)) = edges.get(current) {
                    current = edge.from;
                    out.push(edge.clone());
                } else {
                    panic!("When building path after pathfinding, did not have an edge from {}. This is never expected to happen.", current);
                }
            }
            out.reverse();
            out
        }

        let mut open = vec![false; self.nodes];
        let mut closed = vec![false; self.nodes];
        let mut edges = vec![None; self.nodes];
        let mut heap = BinaryHeap::new();

        heap.push(Node::new(from, None, 0, heuristic));

        while let Some(Node {
            index,
            entry,
            distance_from_start,
            ..
        }) = heap.pop()
        {
            if let Some(max_cost) = max_cost {
                if distance_from_start > max_cost {
                    return None;
                }
            }
            if closed[index] {
                continue;
            }
            edges[index] = entry;
            if index == to {
                return Some(get_path(from, to, &edges));
            }
            open[index] = false;
            closed[index] = true;
            for edge in self.get_out(index) {
                let neighbour = edge.to;
                if closed[neighbour] {
                    continue;
                }
                let neighbour_distance_from_start = distance_from_start + u32::from(edge.cost);
                heap.push(Node::new(
                    neighbour,
                    Some(*edge),
                    neighbour_distance_from_start,
                    heuristic,
                ));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {

    use hamcrest::prelude::*;
    use {Edge, Network};

    fn get_test_edges() -> Vec<Edge> {
        vec![
            Edge::new(0, 1, 1),
            Edge::new(0, 2, 2),
            Edge::new(0, 2, 3),
            Edge::new(1, 3, 4),
            Edge::new(2, 3, 5),
            Edge::new(2, 3, 6),
            Edge::new(5, 6, 7),
            Edge::new(6, 5, 8),
            Edge::new(6, 5, 9),
            Edge::new(7, 7, 10),
        ]
    }

    fn get_test_network(edges: &[Edge]) -> Network {
        Network::new(8, edges)
    }

    #[test]
    fn test_add_edges() {
        let mut network = Network::new(7, &[]);
        network.add_edge(&Edge::new(1, 2, 4));
        network.add_edge(&Edge::new(3, 2, 5));
        assert_eq!(network.get_out(1), &vec![Edge::new(1, 2, 4)]);
        assert_that!(
            network.get_in(2),
            contains(vec![Edge::new(1, 2, 4), Edge::new(3, 2, 5),]).exactly()
        );
        assert_eq!(network.get_out(3), &vec![Edge::new(3, 2, 5)]);
    }

    #[test]
    fn test_remove_edges() {
        let mut network = Network::new(7, &[]);
        network.add_edge(&Edge::new(1, 2, 4));
        network.add_edge(&Edge::new(3, 2, 5));
        network.add_edge(&Edge::new(2, 1, 4));
        network.remove_edges(1, 2);
        assert_eq!(network.get_out(1), &vec![]);
        assert_eq!(network.get_in(2), &vec![Edge::new(3, 2, 5)]);
        assert_eq!(network.get_out(3), &vec![Edge::new(3, 2, 5)]);
        assert_eq!(network.get_out(2), &vec![Edge::new(2, 1, 4)]);
        assert_eq!(network.get_in(1), &vec![Edge::new(2, 1, 4)]);
    }

    #[test]
    fn test_get_out() {
        let edges = get_test_edges();
        let network = get_test_network(&edges);
        assert_that!(
            &network.get_out(0).iter().collect(),
            contains(vec![&edges[0], &edges[1], &edges[2]]).exactly()
        );
        assert_that!(
            &network.get_out(1).iter().collect(),
            contains(vec![&edges[3]]).exactly()
        );
        assert_that!(
            &network.get_out(2).iter().collect(),
            contains(vec![&edges[4], &edges[5]]).exactly()
        );
        assert_that!(network.get_out(3).len(), is(equal_to(0)));
        assert_that!(network.get_out(4).len(), is(equal_to(0)));
        assert_that!(
            &network.get_out(5).iter().collect(),
            contains(vec![&edges[6]]).exactly()
        );
        assert_that!(
            &network.get_out(6).iter().collect(),
            contains(vec![&edges[7], &edges[8]]).exactly()
        );
        assert_that!(
            &network.get_out(7).iter().collect(),
            contains(vec![&edges[9]]).exactly()
        );
    }

    #[test]
    fn test_get_in() {
        let edges = get_test_edges();
        let network = get_test_network(&edges);
        assert_that!(network.get_in(0).len(), is(equal_to(0)));
        assert_that!(
            &network.get_in(1).iter().collect(),
            contains(vec![&edges[0]]).exactly()
        );
        assert_that!(
            &network.get_in(2).iter().collect(),
            contains(vec![&edges[1], &edges[2]]).exactly()
        );
        assert_that!(
            &network.get_in(3).iter().collect(),
            contains(vec![&edges[3], &edges[4], &edges[5]]).exactly()
        );
        assert_that!(network.get_in(4).len(), is(equal_to(0)));
        assert_that!(
            &network.get_in(5).iter().collect(),
            contains(vec![&edges[7], &edges[8]]).exactly()
        );
        assert_that!(
            &network.get_in(6).iter().collect(),
            contains(vec![&edges[6]]).exactly()
        );
        assert_that!(
            &network.get_in(7).iter().collect(),
            contains(vec![&edges[9]]).exactly()
        );
    }

    #[test]
    fn test_create_grid() {
        let expected_edges = vec![
            Edge::new(0, 1, 1),
            Edge::new(0, 3, 1),
            Edge::new(1, 0, 1),
            Edge::new(1, 2, 1),
            Edge::new(1, 4, 1),
            Edge::new(2, 1, 1),
            Edge::new(2, 5, 1),
            Edge::new(3, 0, 1),
            Edge::new(3, 4, 1),
            Edge::new(3, 6, 1),
            Edge::new(4, 1, 1),
            Edge::new(4, 3, 1),
            Edge::new(4, 5, 1),
            Edge::new(4, 7, 1),
            Edge::new(5, 2, 1),
            Edge::new(5, 4, 1),
            Edge::new(5, 8, 1),
            Edge::new(6, 3, 1),
            Edge::new(6, 7, 1),
            Edge::new(7, 4, 1),
            Edge::new(7, 6, 1),
            Edge::new(7, 8, 1),
            Edge::new(8, 5, 1),
            Edge::new(8, 7, 1),
        ];

        let edges = Edge::create_grid(3, 3, 1, Edge::create_4_neighbour_deltas());
        assert_that!(
            &edges.iter().collect(),
            contains(expected_edges.iter().collect()).exactly()
        );
    }

    #[test]
    fn test_dijkstra() {
        let edges = get_test_edges();
        let network = get_test_network(&edges);
        let expected = vec![
            vec![Some(0), None, None, None, None, None, None, None],
            vec![Some(1), Some(0), None, None, None, None, None, None],
            vec![Some(2), None, Some(0), None, None, None, None, None],
            vec![Some(5), Some(4), Some(5), Some(0), None, None, None, None],
            vec![None, None, None, None, Some(0), None, None, None],
            vec![None, None, None, None, None, Some(0), Some(8), None],
            vec![None, None, None, None, None, Some(7), Some(0), None],
            vec![None, None, None, None, None, None, None, Some(0)],
        ];

        for (i, expected) in expected.iter().enumerate() {
            assert_that!(&network.dijkstra(vec![i]), is(equal_to(expected)));
        }
    }

    #[test]
    fn test_dijkstra_on_grid() {
        let edges = Edge::create_grid(4, 4, 1, Edge::create_4_neighbour_deltas());
        let network = Network::new(16, &edges);
        let expected = vec![
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
        ];

        assert_that!(&network.dijkstra(vec![0]), is(equal_to(&expected)));
    }

    #[test]
    fn test_dijkstra_multi_destinations() {
        let edges = Edge::create_grid(4, 4, 1, Edge::create_4_neighbour_deltas());
        let network = Network::new(16, &edges);
        let expected = vec![
            Some(0),
            Some(0),
            Some(0),
            Some(0),
            Some(1),
            Some(1),
            Some(1),
            Some(1),
            Some(2),
            Some(2),
            Some(2),
            Some(2),
            Some(3),
            Some(3),
            Some(3),
            Some(3),
        ];

        assert_that!(&network.dijkstra(vec![0, 1, 2, 3]), is(equal_to(&expected)));
    }

    #[test]
    fn test_find_path_one_way_right_way() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(1, 2, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);

        let actual = network.find_path(0, 3, None, &|_| 0);
        let expected = Some(vec![
            Edge::new(0, 1, 1),
            Edge::new(1, 2, 1),
            Edge::new(2, 3, 1),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_one_way_wrong_way() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(1, 2, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);

        let actual = network.find_path(3, 0, None, &|_| 0);
        let expected = None;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_where_no_path_under_max_cost() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(1, 2, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);

        let actual = network.find_path(0, 3, Some(2), &|_| 0);
        let expected = None;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_where_path_equals_max_cost() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(1, 2, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);

        let actual = network.find_path(0, 3, Some(3), &|_| 0);
        let expected = Some(vec![
            Edge::new(0, 1, 1),
            Edge::new(1, 2, 1),
            Edge::new(2, 3, 1),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_isolated_nodes() {
        let network = Network::new(4, &[]);
        let actual = network.find_path(0, 3, Some(2), &|_| 0);
        let expected = None;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_two_routes() {
        let edges = vec![
            Edge::new(0, 1, 1),
            Edge::new(0, 2, 1),
            Edge::new(1, 3, 1),
            Edge::new(2, 3, 1),
        ];
        let network = Network::new(4, &edges);

        let actual = network.find_path(0, 3, None, &|_| 0);
        let via_1 = Some(vec![Edge::new(0, 1, 1), Edge::new(1, 3, 1)]);
        let via_2 = Some(vec![Edge::new(0, 2, 1), Edge::new(2, 3, 1)]);
        assert!(actual == via_1 || actual == via_2);
    }

    #[test]
    fn test_find_path_where_shortest_path_does_not_have_least_edges() {
        let edges = vec![
            Edge::new(0, 3, 10),
            Edge::new(0, 1, 1),
            Edge::new(1, 2, 1),
            Edge::new(2, 3, 1),
        ];
        let network = Network::new(4, &edges);

        let actual = network.find_path(0, 3, Some(3), &|_| 0);
        let expected = Some(vec![
            Edge::new(0, 1, 1),
            Edge::new(1, 2, 1),
            Edge::new(2, 3, 1),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_on_grid() {
        let mut edges = Edge::create_grid(4, 4, 1, Edge::create_4_neighbour_deltas());
        let to_remove = vec![
            Edge::new(14, 13, 1),
            Edge::new(10, 9, 1),
            Edge::new(10, 6, 1),
        ];
        let edges: Vec<Edge> = edges
            .drain(..)
            .filter(|edge| !to_remove.contains(edge))
            .collect();

        let network = Network::new(16, &edges);
        let actual = network.find_path(10, 13, None, &|_| 0);
        let expected = Some(vec![
            Edge::new(10, 11, 1),
            Edge::new(11, 7, 1),
            Edge::new(7, 6, 1),
            Edge::new(6, 5, 1),
            Edge::new(5, 9, 1),
            Edge::new(9, 13, 1),
        ]);
        assert_eq!(actual, expected);
    }
}
