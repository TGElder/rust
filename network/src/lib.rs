use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::iter::once;
#[cfg(test)]
#[macro_use]
extern crate hamcrest;

#[derive(Eq)]
struct Node {
    index: usize,
    cost: u128,
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

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct NodeWithinResult {
    pub index: usize,
    pub cost: u128,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ClosestTargetResult {
    pub node: usize,
    pub path: Vec<usize>,
    pub cost: u128,
}

#[derive(Eq, PartialEq, Debug)]
pub struct Network {
    pub nodes: usize,
    edges_out: Vec<Vec<Edge>>,
    edges_in: Vec<Vec<Edge>>,
    targets: HashMap<String, Vec<bool>>,
}

impl Network {
    pub fn new(nodes: usize, edges: &[Edge]) -> Network {
        let mut out = Network {
            nodes,
            edges_out: vec![vec![]; nodes],
            edges_in: vec![vec![]; nodes],
            targets: HashMap::default(),
        };

        edges.iter().for_each(|edge| out.add_edge(edge));

        out
    }

    pub fn add_edge(&mut self, edge: &Edge) {
        self.edges_out.get_mut(edge.from).unwrap().push(*edge);
        self.edges_in.get_mut(edge.to).unwrap().push(*edge);
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

    pub fn reset_edges(&mut self) {
        self.edges_out = vec![vec![]; self.nodes];
        self.edges_in = vec![vec![]; self.nodes];
    }

    pub fn get_in(&self, node: usize) -> &Vec<Edge> {
        &self.edges_in[node]
    }

    pub fn get_out(&self, node: usize) -> &Vec<Edge> {
        &self.edges_out[node]
    }

    pub fn init_targets(&mut self, name: String) {
        self.targets.insert(name, vec![false; self.nodes]);
    }

    pub fn load_target(&mut self, name: &str, index: usize, target: bool) {
        self.targets.get_mut(name).unwrap()[index] = target
    }

    pub fn dijkstra(&self, nodes: Vec<usize>) -> Vec<Option<u128>> {
        let mut closed: Vec<bool> = vec![false; self.nodes];
        let mut out: Vec<Option<u128>> = vec![None; self.nodes];
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
                            cost: cost + u128::from(edge.cost),
                        });
                    }
                }
            }
        }

        out
    }

    pub fn find_path(
        &self,
        from: &[usize],
        to: &[usize],
        max_cost: Option<u32>,
        heuristic: &dyn Fn(usize) -> u32,
    ) -> Option<Vec<Edge>> {
        #[derive(Eq)]
        struct AStarNode {
            index: usize,
            entry: Option<Edge>,
            distance_from_start: u32,
            estimated_path_distance_via_this_node: u32,
        }

        impl AStarNode {
            fn new(
                index: usize,
                entry: Option<Edge>,
                distance_from_start: u32,
                heuristic: &dyn Fn(usize) -> u32,
            ) -> AStarNode {
                let estimated_distance_to_goal = heuristic(index);
                AStarNode {
                    index,
                    entry,
                    distance_from_start,
                    estimated_path_distance_via_this_node: distance_from_start
                        + estimated_distance_to_goal,
                }
            }
        }

        impl Ord for AStarNode {
            fn cmp(&self, other: &AStarNode) -> Ordering {
                self.estimated_path_distance_via_this_node
                    .cmp(&other.estimated_path_distance_via_this_node)
                    .reverse()
            }
        }

        impl PartialOrd for AStarNode {
            fn partial_cmp(&self, other: &AStarNode) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl PartialEq for AStarNode {
            fn eq(&self, other: &AStarNode) -> bool {
                self.estimated_path_distance_via_this_node
                    == other.estimated_path_distance_via_this_node
            }
        }

        let mut to_vector = vec![false; self.nodes];
        to.iter().for_each(|to| to_vector[*to] = true);
        let mut closed = vec![false; self.nodes];
        let mut edges = vec![None; self.nodes];
        let mut heap = BinaryHeap::new();

        for from in from.iter() {
            heap.push(AStarNode::new(*from, None, 0, heuristic));
        }

        while let Some(AStarNode {
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
            if to_vector[index] {
                return Some(get_path(from, index, &edges));
            }
            closed[index] = true;
            for edge in self.get_out(index) {
                let neighbour = edge.to;
                if closed[neighbour] {
                    continue;
                }
                let neighbour_distance_from_start = distance_from_start + u32::from(edge.cost);
                heap.push(AStarNode::new(
                    neighbour,
                    Some(*edge),
                    neighbour_distance_from_start,
                    heuristic,
                ));
            }
        }

        None
    }

    pub fn nodes_within(&self, start_nodes: &[usize], max_cost: u128) -> Vec<NodeWithinResult> {
        let mut closed = vec![false; self.nodes];
        let mut heap = BinaryHeap::new();
        let mut out = vec![];
        for node in start_nodes {
            heap.push(Node {
                index: *node,
                cost: 0,
            });
        }

        while let Some(Node { index, cost, .. }) = heap.pop() {
            if cost > max_cost {
                break;
            }
            if closed[index] {
                continue;
            }
            closed[index] = true;
            out.push(NodeWithinResult { index, cost });
            for edge in self.get_out(index) {
                let neighbour = edge.to;
                if closed[neighbour] {
                    continue;
                }
                heap.push(Node {
                    index: neighbour,
                    cost: cost + u128::from(edge.cost),
                });
            }
        }

        out
    }

    pub fn closest_targets(
        &self,
        start_nodes: &[usize],
        targets: &[bool],
        n_closest: usize,
    ) -> Vec<ClosestTargetResult> {
        #[derive(Eq)]
        struct CTNode {
            index: usize,
            cost: u128,
            entry: Option<Edge>,
        }

        impl CTNode {
            fn new(index: usize, cost: u128, entry: Option<Edge>) -> CTNode {
                CTNode { index, cost, entry }
            }
        }

        impl Ord for CTNode {
            fn cmp(&self, other: &CTNode) -> Ordering {
                self.cost.cmp(&other.cost).reverse()
            }
        }

        impl PartialOrd for CTNode {
            fn partial_cmp(&self, other: &CTNode) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl PartialEq for CTNode {
            fn eq(&self, other: &CTNode) -> bool {
                self.cost == other.cost
            }
        }

        if targets.len() != self.nodes {
            panic!("Length of target slice must equal size of network");
        }

        if n_closest == 0 {
            return vec![];
        }

        let mut closed = vec![false; self.nodes];
        let mut edges = vec![None; self.nodes];
        let mut heap = BinaryHeap::new();
        let mut out = vec![];
        let mut last_cost = None;

        for node in start_nodes {
            heap.push(CTNode::new(*node, 0, None))
        }

        while let Some(CTNode { index, cost, entry }) = heap.pop() {
            if closed[index] {
                continue;
            }
            edges[index] = entry;
            if targets[index] {
                if let Some(last_cost) = last_cost {
                    if out.len() >= n_closest && last_cost < cost {
                        return out;
                    }
                }
                last_cost = Some(cost);
                out.push(ClosestTargetResult {
                    node: index,
                    cost,
                    path: get_path(start_nodes, index, &edges)
                        .drain(..)
                        .map(|edge| edge.from)
                        .chain(once(index))
                        .collect(),
                });
            }
            closed[index] = true;
            for edge in self.get_out(index) {
                let neighbour = edge.to;
                if closed[neighbour] {
                    continue;
                }
                heap.push(CTNode {
                    index: neighbour,
                    cost: cost + u128::from(edge.cost),
                    entry: Some(*edge),
                });
            }
        }

        out
    }

    pub fn closest_loaded_targets(
        &self,
        start_nodes: &[usize],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult> {
        self.closest_targets(start_nodes, &self.targets[targets], n_closest)
    }
}

fn get_path(from: &[usize], to: usize, edges: &[Option<Edge>]) -> Vec<Edge> {
    let mut out = vec![];
    let mut current = to;
    while !from.contains(&current) {
        if let Some(Some(edge)) = edges.get(current) {
            current = edge.from;
            out.push(*edge);
        } else {
            panic!("When building path after pathfinding, did not have an edge from {}. This is never expected to happen.", current);
        }
    }
    out.reverse();
    out
}

#[cfg(test)]
mod tests {

    use super::*;
    use hamcrest::prelude::*;

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

        let actual = network.find_path(&[0], &[3], None, &|_| 0);
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

        let actual = network.find_path(&[3], &[0], None, &|_| 0);
        let expected = None;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_where_no_path_under_max_cost() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(1, 2, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);

        let actual = network.find_path(&[0], &[3], Some(2), &|_| 0);
        let expected = None;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_where_path_equals_max_cost() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(1, 2, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);

        let actual = network.find_path(&[0], &[3], Some(3), &|_| 0);
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
        let actual = network.find_path(&[0], &[3], Some(2), &|_| 0);
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

        let actual = network.find_path(&[0], &[3], None, &|_| 0);
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

        let actual = network.find_path(&[0], &[3], Some(3), &|_| 0);
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
        let actual = network.find_path(&[10], &[13], None, &|_| 0);
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

    #[test]
    fn test_find_path_multiple_from() {
        let edges = vec![
            Edge::new(1, 0, 1),
            Edge::new(2, 1, 1),
            Edge::new(3, 2, 2),
            Edge::new(4, 0, 1),
            Edge::new(5, 4, 1),
            Edge::new(6, 5, 1),
        ];
        let network = Network::new(7, &edges);

        let actual = network.find_path(&[3, 6], &[0], None, &|_| 0);
        let expected = Some(vec![
            Edge::new(6, 5, 1),
            Edge::new(5, 4, 1),
            Edge::new(4, 0, 1),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_multiple_to() {
        let edges = vec![
            Edge::new(0, 1, 1),
            Edge::new(1, 2, 1),
            Edge::new(2, 3, 2),
            Edge::new(0, 4, 1),
            Edge::new(4, 5, 1),
            Edge::new(5, 6, 1),
        ];
        let network = Network::new(7, &edges);

        let actual = network.find_path(&[0], &[3, 6], None, &|_| 0);
        let expected = Some(vec![
            Edge::new(0, 4, 1),
            Edge::new(4, 5, 1),
            Edge::new(5, 6, 1),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_from_equals_to() {
        let edges = vec![Edge::new(0, 1, 1)];
        let network = Network::new(2, &edges);

        let actual = network.find_path(&[0], &[0], None, &|_| 0);
        let expected = Some(vec![]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_path_from_overlaps_to() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(1, 2, 1)];
        let network = Network::new(3, &edges);

        let actual = network.find_path(&[0, 1], &[1, 2], None, &|_| 0);
        let expected = Some(vec![]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_nodes_within() {
        let edges = vec![
            Edge::new(0, 1, 1),
            Edge::new(1, 2, 2),
            Edge::new(0, 3, 10),
            Edge::new(3, 4, 1),
            Edge::new(0, 5, 1),
            Edge::new(5, 6, 10),
            Edge::new(0, 7, 1),
            Edge::new(7, 8, 10),
            Edge::new(0, 8, 2),
        ];
        let network = Network::new(9, &edges);
        let actual = network.nodes_within(&[0], 10);
        let expected = vec![
            NodeWithinResult { index: 0, cost: 0 },
            NodeWithinResult { index: 1, cost: 1 },
            NodeWithinResult { index: 2, cost: 3 },
            NodeWithinResult { index: 3, cost: 10 },
            NodeWithinResult { index: 5, cost: 1 },
            NodeWithinResult { index: 7, cost: 1 },
            NodeWithinResult { index: 8, cost: 2 },
        ];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_nodes_within_multiple_start_nodes() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);
        let actual = network.nodes_within(&[0, 2], 1);
        let expected = vec![
            NodeWithinResult { index: 0, cost: 0 },
            NodeWithinResult { index: 1, cost: 1 },
            NodeWithinResult { index: 2, cost: 0 },
            NodeWithinResult { index: 3, cost: 1 },
        ];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_closest_targets_no_closest_targets() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(0, 2, 1)];
        let network = Network::new(4, &edges);
        let actual = network.closest_targets(&[0], &[false, false, false, true], 1);
        assert!(actual.is_empty());
    }

    #[test]
    fn test_closest_targets_with_tie() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(0, 2, 2)];
        let network = Network::new(3, &edges);
        let actual = network.closest_targets(&[0], &[false, true, true], 1);
        let expected = vec![ClosestTargetResult {
            node: 1,
            path: vec![0, 1],
            cost: 1,
        }];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_closest_targets_multiple_closest_targets() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(0, 2, 1)];
        let network = Network::new(3, &edges);
        let actual = network.closest_targets(&[0], &[false, true, true], 1);
        let expected = vec![
            ClosestTargetResult {
                node: 1,
                path: vec![0, 1],
                cost: 1,
            },
            ClosestTargetResult {
                node: 2,
                path: vec![0, 2],
                cost: 1,
            },
        ];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_closest_targets_closest_target_more_edges() {
        let edges = vec![Edge::new(0, 1, 3), Edge::new(0, 2, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);
        let actual = network.closest_targets(&[0], &[false, true, false, true], 1);
        let expected = vec![ClosestTargetResult {
            node: 3,
            path: vec![0, 2, 3],
            cost: 2,
        }];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_closest_targets_start_node_is_target() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(0, 2, 2)];
        let network = Network::new(3, &edges);
        let actual = network.closest_targets(&[0], &[true, true, true], 1);
        let expected = vec![ClosestTargetResult {
            node: 0,
            path: vec![0],
            cost: 0,
        }];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_closest_targets_multiple_start_nodes() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(2, 3, 1)];
        let network = Network::new(4, &edges);
        let actual = network.closest_targets(&[0, 2], &[false, true, false, true], 1);
        let expected = vec![
            ClosestTargetResult {
                node: 1,
                path: vec![0, 1],
                cost: 1,
            },
            ClosestTargetResult {
                node: 3,
                path: vec![2, 3],
                cost: 1,
            },
        ];

        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_multiple_closest_targets() {
        let edges = vec![
            Edge::new(0, 1, 1),
            Edge::new(1, 2, 1),
            Edge::new(0, 3, 2),
            Edge::new(3, 4, 2),
        ];
        let network = Network::new(5, &edges);
        let actual = network.closest_targets(&[0], &[false, true, true, true, true], 3);
        let expected = vec![
            ClosestTargetResult {
                node: 1,
                path: vec![0, 1],
                cost: 1,
            },
            ClosestTargetResult {
                node: 2,
                path: vec![0, 1, 2],
                cost: 2,
            },
            ClosestTargetResult {
                node: 3,
                path: vec![0, 3],
                cost: 2,
            },
        ];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_multiple_closest_targets_with_tie() {
        let edges = vec![
            Edge::new(0, 1, 1),
            Edge::new(1, 2, 1),
            Edge::new(0, 3, 2),
            Edge::new(3, 4, 2),
        ];
        let network = Network::new(5, &edges);
        let actual = network.closest_targets(&[0], &[false, true, true, true, true], 2);
        let expected = vec![
            ClosestTargetResult {
                node: 1,
                path: vec![0, 1],
                cost: 1,
            },
            ClosestTargetResult {
                node: 2,
                path: vec![0, 1, 2],
                cost: 2,
            },
            ClosestTargetResult {
                node: 3,
                path: vec![0, 3],
                cost: 2,
            },
        ];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    fn test_closest_targets_zero_targets() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(0, 2, 1)];
        let network = Network::new(4, &edges);
        let actual = network.closest_targets(&[0], &[false, true, true, true], 0);
        assert!(actual.is_empty());
    }

    #[test]
    #[should_panic]
    fn test_closest_targets_wrong_number_of_targets() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(0, 2, 1)];
        let network = Network::new(3, &edges);
        network.closest_targets(&[0, 2], &[false, true, false, true], 1);
    }

    #[test]
    fn test_closest_loaded_targets_via_load_target() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(0, 2, 2)];
        let mut network = Network::new(3, &edges);
        network.init_targets(String::from("targets"));
        network.load_target("targets", 1, true);
        network.load_target("targets", 2, true);
        let actual = network.closest_loaded_targets(&[0], "targets", 1);
        let expected = vec![ClosestTargetResult {
            node: 1,
            path: vec![0, 1],
            cost: 1,
        }];
        assert_that!(&actual, contains(expected).exactly());
    }

    #[test]
    #[should_panic]
    fn test_closest_loaded_targets_unknown_name() {
        let edges = vec![Edge::new(0, 1, 1), Edge::new(0, 2, 2)];
        let network = Network::new(3, &edges);
        network.closest_loaded_targets(&[0], "targets", 1);
    }
}
