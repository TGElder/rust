use std::cmp;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::Hash;

use crate::Network;

pub trait ClosestOrigins {
    fn closest_origins<T: Copy + Eq + Hash>(
        &self,
        origin_to_indices: &HashMap<T, Vec<usize>>,
    ) -> Vec<HashSet<T>>;
}

impl ClosestOrigins for Network {
    fn closest_origins<T: Copy + Eq + Hash>(
        &self,
        origin_to_indices: &HashMap<T, Vec<usize>>,
    ) -> Vec<HashSet<T>> {
        let mut out = vec![HashSet::default(); self.nodes];

        if origin_to_indices.is_empty() {
            return out;
        }

        let mut heap: BinaryHeap<Node<T>> = BinaryHeap::new();
        let mut min_costs = vec![None; self.nodes];

        for (origin, indices) in origin_to_indices {
            for index in indices {
                heap.push(Node {
                    index: *index,
                    cost: 0,
                    origin: *origin,
                });
            }
        }

        while let Some(Node {
            index,
            cost,
            origin,
        }) = heap.pop()
        {
            if let Some(min_cost) = min_costs[index] {
                if cost != min_cost {
                    continue;
                }
            } else {
                min_costs[index] = Some(cost);
            }

            out[index].insert(origin);

            for edge in self.get_out(&index) {
                let neighbour = edge.to;
                let cost = cost + u128::from(edge.cost);

                heap.push(Node {
                    index: neighbour,
                    cost,
                    origin,
                });
            }
        }

        out
    }
}

struct Node<T> {
    index: usize,
    cost: u128,
    origin: T,
}

impl<T> Ord for Node<T> {
    fn cmp(&self, other: &Node<T>) -> cmp::Ordering {
        self.cost.cmp(&other.cost).reverse()
    }
}

impl<T> PartialOrd for Node<T> {
    fn partial_cmp(&self, other: &Node<T>) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq for Node<T> {
    fn eq(&self, other: &Node<T>) -> bool {
        self.cost == other.cost
    }
}

impl<T> Eq for Node<T> {}

#[cfg(test)]
mod tests {
    use maplit::{hashmap, hashset};

    use crate::Edge;

    use super::*;

    #[test]
    fn test() {
        // Given
        let network = Network::new(
            9,
            &[
                Edge::new(0, 1, 1),
                Edge::new(0, 2, 1),
                Edge::new(0, 3, 1),
                Edge::new(1, 2, 1),
                Edge::new(1, 4, 1),
                Edge::new(3, 5, 1),
                Edge::new(3, 6, 1),
                Edge::new(4, 5, 2),
                Edge::new(4, 6, 2),
                Edge::new(7, 6, 1),
            ],
        );

        // When
        let actual = network.closest_origins(&hashmap! {
            "a" => vec![0],
            "b" => vec![1, 7],
        });

        // Then
        assert_eq!(
            actual,
            vec![
                hashset! {"a"},
                hashset! {"b"},
                hashset! {"a", "b"},
                hashset! {"a"},
                hashset! {"b"},
                hashset! {"a"},
                hashset! {"b"},
                hashset! {"b"},
                hashset! {},
            ]
        );
    }
}
