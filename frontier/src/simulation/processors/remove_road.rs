use std::collections::HashSet;

use commons::edge::Edge;

use crate::build::BuildKey;
use crate::route::RouteKey;
use crate::traits::{
    IsRoad, PlanRoad, RemoveBuildInstruction, RemoveRoad as RemoveRoadTrait, RoadPlanned,
    WithEdgeTraffic,
};

use super::*;

pub struct RemoveRoad<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for RemoveRoad<T>
where
    T: IsRoad
        + PlanRoad
        + RemoveBuildInstruction
        + RemoveRoadTrait
        + RoadPlanned
        + WithEdgeTraffic
        + Send
        + Sync
        + 'static,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let edges = match instruction {
            Instruction::RefreshEdges(edges) => edges,
            _ => return state,
        };

        for edge in self.get_edges_with_no_traffic(edges).await {
            self.process_edge(edge).await;
        }

        state
    }
}

impl<T> RemoveRoad<T>
where
    T: IsRoad + PlanRoad + RemoveBuildInstruction + RemoveRoadTrait + RoadPlanned + WithEdgeTraffic,
{
    pub fn new(tx: T) -> RemoveRoad<T> {
        RemoveRoad { tx }
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    async fn get_edges_with_no_traffic<'a>(&self, edges: &'a HashSet<Edge>) -> HashSet<&'a Edge> {
        self.tx
            .with_edge_traffic(|edge_traffic| {
                edges
                    .iter()
                    .filter(|edge| traffic_is_empty(edge_traffic.get(edge)))
                    .collect()
            })
            .await
    }

    async fn process_edge(&mut self, edge: &Edge) {
        if !self.tx.is_road(*edge).await && self.tx.road_planned(*edge).await.is_none() {
            return;
        }

        self.tx
            .remove_build_instruction(&BuildKey::Road(*edge))
            .await;
        self.tx.remove_road(*edge).await;
        self.tx.plan_road(*edge, None).await;
    }
}

fn traffic_is_empty(traffic: Option<&HashSet<RouteKey>>) -> bool {
    match traffic {
        Some(traffic) => traffic.is_empty(),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Mutex;

    use commons::v2;
    use futures::executor::block_on;

    use crate::resource::Resource;
    use crate::route::RouteKey;

    use super::*;

    #[derive(Default)]
    struct Tx {
        is_road: bool,
        edge_traffic: Mutex<EdgeTraffic>,
        planned_roads: Mutex<Vec<(Edge, Option<u128>)>>,
        removed_build_instructions: Mutex<HashSet<BuildKey>>,
        removed_roads: Mutex<Vec<Edge>>,
        road_planned: Option<u128>,
    }

    #[async_trait]
    impl IsRoad for Tx {
        async fn is_road(&self, _: Edge) -> bool {
            self.is_road
        }
    }

    #[async_trait]
    impl PlanRoad for Tx {
        async fn plan_road(&self, edge: Edge, when: Option<u128>) {
            self.planned_roads.lock().unwrap().push((edge, when))
        }
    }

    #[async_trait]
    impl RemoveBuildInstruction for Tx {
        async fn remove_build_instruction(&self, build_key: &BuildKey) {
            self.removed_build_instructions
                .lock()
                .unwrap()
                .insert(*build_key);
        }
    }

    #[async_trait]
    impl RemoveRoadTrait for Tx {
        async fn remove_road(&self, edge: Edge) {
            self.removed_roads.lock().unwrap().push(edge)
        }
    }

    #[async_trait]
    impl RoadPlanned for Tx {
        async fn road_planned(&self, _: Edge) -> Option<u128> {
            self.road_planned
        }
    }

    #[async_trait]
    impl WithEdgeTraffic for Tx {
        async fn with_edge_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&EdgeTraffic) -> O + Send,
        {
            function(&self.edge_traffic.lock().unwrap())
        }

        async fn mut_edge_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut EdgeTraffic) -> O + Send,
        {
            function(&mut self.edge_traffic.lock().unwrap())
        }
    }

    #[test]
    fn should_remove_existing_road() {
        // Given
        let tx = Tx {
            is_road: true,
            ..Tx::default()
        };
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(
            State::default(),
            &Instruction::RefreshEdges(hashset! {edge}),
        ));

        // Then
        assert_eq!(*processor.tx.removed_roads.lock().unwrap(), vec![edge]);
    }

    #[test]
    fn should_remove_planned_road() {
        // Given
        let tx = Tx {
            road_planned: Some(123),
            ..Tx::default()
        };
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(
            State::default(),
            &Instruction::RefreshEdges(hashset! {edge}),
        ));

        // Then
        assert_eq!(
            *processor.tx.planned_roads.lock().unwrap(),
            vec![(edge, None)]
        );
    }

    #[test]
    fn should_remove_build_instruction() {
        // Given
        let tx = Tx {
            road_planned: Some(123),
            ..Tx::default()
        };

        let edge = Edge::new(v2(0, 0), v2(1, 0));

        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(
            State::default(),
            &Instruction::RefreshEdges(hashset! {edge}),
        ));

        // Then
        assert_eq!(
            *processor.tx.removed_build_instructions.lock().unwrap(),
            hashset! {BuildKey::Road(edge)}
        );
    }

    #[test]
    fn should_not_remove_if_any_routes() {
        // Given
        let tx = Tx {
            is_road: true,
            road_planned: Some(123),
            ..Tx::default()
        };

        let edge = Edge::new(v2(0, 0), v2(1, 0));
        *tx.edge_traffic.lock().unwrap() = hashmap! {
            edge => hashset!{
                RouteKey{
                    settlement: v2(0, 0),
                    resource: Resource::Coal,
                    destination: v2(1, 1),
                }
            }
        };

        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(
            State::default(),
            &Instruction::RefreshEdges(hashset! {edge}),
        ));

        // Then
        assert!(processor
            .tx
            .removed_build_instructions
            .lock()
            .unwrap()
            .is_empty());
        assert!(processor.tx.removed_roads.lock().unwrap().is_empty());
        assert!(processor.tx.planned_roads.lock().unwrap().is_empty());
    }

    #[test]
    fn should_remove_if_empty_route_entry() {
        // Given
        let tx = Tx {
            is_road: true,
            ..Tx::default()
        };
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        *tx.edge_traffic.lock().unwrap() = hashmap! {
            edge => hashset!{}
        };
        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(
            State::default(),
            &Instruction::RefreshEdges(hashset! {edge}),
        ));

        // Then
        assert_eq!(*processor.tx.removed_roads.lock().unwrap(), vec![edge]);
    }

    #[test]
    fn should_not_remove_if_road_neither_exists_nor_planned() {
        // Given
        let tx = Tx {
            is_road: false,
            road_planned: None,
            ..Tx::default()
        };
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(
            State::default(),
            &Instruction::RefreshEdges(hashset! {edge}),
        ));

        // Then
        assert_eq!(*processor.tx.removed_roads.lock().unwrap(), vec![]);
        assert_eq!(*processor.tx.planned_roads.lock().unwrap(), vec![]);
    }
}
