use commons::edge::Edge;

use crate::traits::{IsRoad, PlanRoad, RemoveRoad as RemoveRoadTrait, RoadPlanned};

use super::*;

pub struct RemoveRoad<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for RemoveRoad<T>
where
    T: IsRoad + PlanRoad + RemoveRoadTrait + RoadPlanned + Send + Sync + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let edges = match instruction {
            Instruction::RefreshEdges(edges) => edges.clone(),
            _ => return state,
        };

        for edge in edges {
            self.process_edge(&mut state, edge).await;
        }

        state
    }
}

impl<T> RemoveRoad<T>
where
    T: IsRoad + PlanRoad + RemoveRoadTrait + RoadPlanned,
{
    pub fn new(tx: T) -> RemoveRoad<T> {
        RemoveRoad { tx }
    }

    async fn process_edge(&mut self, state: &mut State, edge: Edge) {
        if !state
            .edge_traffic
            .get(&edge)
            .map_or(true, |routes| routes.is_empty())
        {
            return;
        }

        if !self.tx.is_road(edge).await && self.tx.road_planned(edge).await.is_none() {
            return;
        }

        state.build_queue.remove(&BuildKey::Road(edge));
        self.tx.remove_road(edge).await;
        self.tx.plan_road(edge, None).await;
    }
}

#[cfg(test)]
mod tests {
    use commons::{v2, Arm};
    use futures::executor::block_on;

    use crate::resource::Resource;
    use crate::route::RouteKey;

    use super::*;

    #[derive(Default)]
    struct Tx {
        is_road: bool,
        planned_roads: Arm<Vec<(Edge, Option<u128>)>>,
        removed_roads: Arm<Vec<Edge>>,
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

    #[test]
    fn should_remove_existing_road() {
        // Given
        let tx = Tx {
            is_road: true,
            ..Tx::default()
        };
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let state = State::default();
        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(state, &Instruction::RefreshEdges(hashset! {edge})));

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
        let state = State::default();
        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(state, &Instruction::RefreshEdges(hashset! {edge})));

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

        let mut build_queue = BuildQueue::default();
        build_queue.insert(BuildInstruction {
            what: Build::Road(edge),
            when: 123,
        });
        let state = State {
            build_queue,
            ..State::default()
        };

        let mut processor = RemoveRoad::new(tx);

        // When
        let state = block_on(processor.process(state, &Instruction::RefreshEdges(hashset! {edge})));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
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

        let mut build_queue = BuildQueue::default();
        build_queue.insert(BuildInstruction {
            what: Build::Road(edge),
            when: 123,
        });
        let state = State {
            build_queue,
            edge_traffic: hashmap! {
                edge => hashset!{
                    RouteKey{
                        settlement: v2(0, 0),
                        resource: Resource::Coal,
                        destination: v2(1, 1),
                    }
                }
            },
            ..State::default()
        };

        let mut processor = RemoveRoad::new(tx);

        // When
        let state = block_on(processor.process(state, &Instruction::RefreshEdges(hashset! {edge})));

        // Then
        assert!(state.build_queue.get(&BuildKey::Road(edge)).is_some());
        assert_eq!(*processor.tx.removed_roads.lock().unwrap(), vec![]);
        assert_eq!(*processor.tx.planned_roads.lock().unwrap(), vec![]);
    }

    #[test]
    fn should_remove_if_empty_route_entry() {
        // Given
        let tx = Tx {
            is_road: true,
            ..Tx::default()
        };
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let state = State {
            edge_traffic: hashmap! {
                edge => hashset!{}
            },
            ..State::default()
        };
        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(state, &Instruction::RefreshEdges(hashset! {edge})));

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
        let state = State::default();
        let mut processor = RemoveRoad::new(tx);

        // When
        block_on(processor.process(state, &Instruction::RefreshEdges(hashset! {edge})));

        // Then
        assert_eq!(*processor.tx.removed_roads.lock().unwrap(), vec![]);
        assert_eq!(*processor.tx.planned_roads.lock().unwrap(), vec![]);
    }
}
