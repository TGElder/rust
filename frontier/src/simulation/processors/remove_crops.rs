use std::collections::HashSet;

use commons::grid::Grid;

use crate::resource::Resource;
use crate::traits::{RemoveWorldObject, SendWorld};
use crate::world::{World, WorldCell, WorldObject};

use super::*;
pub struct RemoveCrops<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for RemoveCrops<T>
where
    T: RemoveWorldObject + SendWorld + Send + Sync + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let mut positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };

        positions.retain(|position| !has_crop_routes(&state, position));

        for position in have_crops_build_instruction(&state, &positions) {
            state.build_queue.remove(&BuildKey::Crops(position));
        }

        for position in self.have_crops(positions).await {
            self.tx.remove_world_object(position).await;
        }

        state
    }
}

impl<T> RemoveCrops<T>
where
    T: RemoveWorldObject + SendWorld,
{
    pub fn new(tx: T) -> RemoveCrops<T> {
        RemoveCrops { tx }
    }

    async fn have_crops(&self, positions: HashSet<V2<usize>>) -> Vec<V2<usize>> {
        self.tx
            .send_world(move |world| have_crops(world, positions))
            .await
    }
}

fn has_crop_routes(state: &State, position: &V2<usize>) -> bool {
    ok_or!(state.traffic.get(&position), return false)
        .iter()
        .any(|route| route.resource == Resource::Crops && route.destination == *position)
}

fn have_crops_build_instruction(state: &State, positions: &HashSet<V2<usize>>) -> Vec<V2<usize>> {
    positions
        .iter()
        .filter(|position| has_crops_build_instruction(state, position))
        .cloned()
        .collect()
}

fn has_crops_build_instruction(state: &State, position: &V2<usize>) -> bool {
    state.build_queue.get(&BuildKey::Crops(*position)).is_some()
}

fn have_crops(world: &World, positions: HashSet<V2<usize>>) -> Vec<V2<usize>> {
    positions
        .into_iter()
        .filter(|position| has_crops(world, position))
        .collect()
}

fn has_crops(world: &World, position: &V2<usize>) -> bool {
    matches!(
        world.get_cell(&position),
        Some(WorldCell {
            object: WorldObject::Crop { .. },
            ..
        })
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use commons::{v2, Arm, M};
    use futures::executor::block_on;

    use crate::route::RouteKey;

    use super::*;

    struct Tx {
        removed_world_objects: Arm<Vec<V2<usize>>>,
        world: Arm<World>,
    }

    #[async_trait]
    impl RemoveWorldObject for Tx {
        async fn remove_world_object(&self, position: V2<usize>) {
            self.removed_world_objects.lock().unwrap().push(position);
        }
    }

    #[async_trait]
    impl SendWorld for Tx {
        async fn send_world<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut World) -> O + Send + 'static,
        {
            function(&mut self.world.lock().unwrap())
        }

        fn send_world_background<F, O>(&self, function: F)
        where
            O: Send + 'static,
            F: FnOnce(&mut World) -> O + Send + 'static,
        {
            function(&mut self.world.lock().unwrap());
        }
    }

    fn happy_path_tx() -> Tx {
        let mut world = World::new(M::zeros(3, 3), 0.0);
        world.mut_cell_unsafe(&v2(1, 1)).object = WorldObject::Crop { rotated: true };
        Tx {
            removed_world_objects: Arm::default(),
            world: Arc::new(Mutex::new(world)),
        }
    }

    fn happy_path_state() -> State {
        State {
            traffic: Traffic::new(3, 3, HashSet::new()),
            ..State::default()
        }
    }

    #[test]
    fn should_remove_crops_if_no_traffic() {
        // Given
        let tx = happy_path_tx();
        let mut processor = RemoveCrops::new(tx);

        // When
        block_on(processor.process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert_eq!(
            *processor.tx.removed_world_objects.lock().unwrap(),
            vec![v2(1, 1)]
        );
    }

    #[test]
    fn should_remove_crops_if_non_crop_traffic() {
        // Given
        let tx = happy_path_tx();

        let mut state = happy_path_state();
        state
            .traffic
            .set(
                &v2(1, 1),
                hashset! {
                    RouteKey{
                        settlement: v2(0, 0),
                        resource: Resource::Pasture,
                        destination: v2(1, 1),
                    }
                },
            )
            .unwrap();

        let mut processor = RemoveCrops::new(tx);

        // When
        block_on(processor.process(state, &Instruction::RefreshPositions(hashset! {v2(1, 1)})));

        // Then
        assert_eq!(
            *processor.tx.removed_world_objects.lock().unwrap(),
            vec![v2(1, 1)]
        );
    }

    #[test]
    fn should_remove_instruction_from_build_queue() {
        // Given
        let tx = happy_path_tx();
        tx.world.lock().unwrap().mut_cell_unsafe(&v2(1, 1)).object = WorldObject::None;

        let mut state = happy_path_state();
        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: v2(1, 1),
                rotated: true,
            },
            when: 1,
        });

        let mut processor = RemoveCrops::new(tx);

        // When
        let state =
            block_on(processor.process(state, &Instruction::RefreshPositions(hashset! {v2(1, 1)})));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_remove_crops_if_crop_traffic() {
        // Given
        let tx = happy_path_tx();

        let mut state = happy_path_state();
        state
            .traffic
            .set(
                &v2(1, 1),
                hashset! {
                    RouteKey{
                        settlement: v2(0, 0),
                        resource: Resource::Crops,
                        destination: v2(1, 1),
                    }
                },
            )
            .unwrap();

        let mut processor = RemoveCrops::new(tx);

        // When
        block_on(processor.process(state, &Instruction::RefreshPositions(hashset! {v2(1, 1)})));

        // Then
        assert!(processor
            .tx
            .removed_world_objects
            .lock()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn should_not_remove_build_instruction_if_crop_traffic() {
        // Given
        let tx = happy_path_tx();
        tx.world.lock().unwrap().mut_cell_unsafe(&v2(1, 1)).object = WorldObject::None;

        let mut state = happy_path_state();
        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: v2(1, 1),
                rotated: true,
            },
            when: 1,
        });
        state
            .traffic
            .set(
                &v2(1, 1),
                hashset! {
                    RouteKey{
                        settlement: v2(0, 0),
                        resource: Resource::Crops,
                        destination: v2(1, 1),
                    }
                },
            )
            .unwrap();

        let mut processor = RemoveCrops::new(tx);

        // When
        let state =
            block_on(processor.process(state, &Instruction::RefreshPositions(hashset! {v2(1, 1)})));

        // Then
        assert!(state.build_queue.get(&BuildKey::Crops(v2(1, 1))).is_some());
    }
}