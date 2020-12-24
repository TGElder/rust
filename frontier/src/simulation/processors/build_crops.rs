use std::collections::HashSet;

use commons::grid::Grid;
use commons::rand::prelude::SmallRng;
use commons::rand::{Rng, SeedableRng};

use crate::game::traits::GetRoute;
use crate::resource::Resource;
use crate::route::Route;
use crate::traits::{SendRoutes, SendWorld};
use crate::world::{World, WorldObject};

use super::*;
pub struct BuildCrops<T> {
    tx: T,
    rng: SmallRng,
}

#[async_trait]
impl<T> Processor for BuildCrops<T>
where
    T: SendRoutes + SendWorld + Send + Sync + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let mut positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };

        positions.retain(|position| has_crop_routes(&state, position));
        let free_positions = self
            .tx
            .send_world(move |world| free_positions(world, positions))
            .await;

        for position in free_positions {
            self.build_crops(&mut state, &position).await;
        }

        state
    }
}

impl<T> BuildCrops<T>
where
    T: SendRoutes,
{
    pub fn new(tx: T, seed: u64) -> BuildCrops<T> {
        BuildCrops {
            tx,
            rng: SeedableRng::seed_from_u64(seed),
        }
    }

    async fn build_crops(&mut self, state: &mut State, position: &V2<usize>) {
        let mut route_keys = ok_or!(state.traffic.get(position), return).clone();
        route_keys.retain(|route| route.resource == Resource::Crops);

        let routes: Vec<Route> = self
            .tx
            .send_routes(move |routes| {
                route_keys
                    .into_iter()
                    .flat_map(|route_key| routes.get_route(&route_key))
                    .cloned()
                    .collect()
            })
            .await;

        let first_visit = routes
            .into_iter()
            .map(|route| route.start_micros + route.duration.as_micros())
            .min()
            .unwrap();

        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: *position,
                rotated: self.rng.gen(),
            },
            when: first_visit,
        });
    }
}

fn has_crop_routes(state: &State, position: &V2<usize>) -> bool {
    ok_or!(state.traffic.get(&position), return false)
        .iter()
        .any(|route| route.resource == Resource::Crops && route.destination == *position)
}

fn free_positions(world: &World, positions: HashSet<V2<usize>>) -> Vec<V2<usize>> {
    positions
        .into_iter()
        .filter(|position| is_free(world, position))
        .collect()
}

fn is_free(world: &World, position: &V2<usize>) -> bool {
    world
        .get_cell(&position)
        .map_or(false, |cell| cell.object == WorldObject::None)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use commons::{v2, Arm, M};
    use futures::executor::block_on;

    use crate::route::{RouteKey, Routes, RoutesExt};

    use super::*;

    struct Tx {
        routes: Arm<Routes>,
        world: Arm<World>,
    }

    #[async_trait]
    impl SendRoutes for Tx {
        async fn send_routes<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut Routes) -> O + Send + 'static,
        {
            function(&mut self.routes.lock().unwrap())
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

    fn happy_path_route_key() -> RouteKey {
        RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Crops,
            destination: v2(1, 1),
        }
    }

    fn happy_path_tx() -> Tx {
        let mut routes = Routes::default();
        routes.insert_route(
            happy_path_route_key(),
            Route {
                path: vec![v2(0, 0), v2(1, 0), v2(1, 1)],
                start_micros: 1,
                duration: Duration::from_micros(10),
                traffic: 7,
            },
        );

        let world = World::new(M::from_element(3, 3, 1.0), 0.5);

        Tx {
            routes: Arc::new(Mutex::new(routes)),
            world: Arc::new(Mutex::new(world)),
        }
    }

    fn happy_path_state() -> State {
        let mut traffic = Traffic::new(3, 3, HashSet::new());
        traffic
            .set(&v2(1, 1), hashset! {happy_path_route_key()})
            .unwrap();
        State {
            traffic,
            ..State::default()
        }
    }

    #[test]
    fn should_build_crops_if_crop_route_ends_at_position() {
        // When
        let state = block_on(BuildCrops::new(happy_path_tx(), 0).process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert!(matches!(
            state.build_queue.get(&BuildKey::Crops(v2(1, 1))),
            Some(BuildInstruction{
                what: Build::Crops{position, .. },
                when: 11
            }) if *position == v2(1, 1)
        ));
    }

    #[test]
    fn should_not_build_crops_if_non_crop_route_ending_at_position() {
        // Given
        let tx = happy_path_tx();
        {
            tx.routes.lock().unwrap().insert_route(
                RouteKey {
                    settlement: v2(2, 2),
                    resource: Resource::Deer,
                    destination: v2(1, 2),
                },
                Route {
                    path: vec![v2(2, 2), v2(1, 2)],
                    start_micros: 1,
                    duration: Duration::from_micros(10),
                    traffic: 7,
                },
            );
        }

        // When
        let state = block_on(BuildCrops::new(tx, 0).process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 2)}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_crops_if_crop_route_not_ending_at_position() {
        // When
        let state = block_on(BuildCrops::new(happy_path_tx(), 0).process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 0)}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_crops_if_cell_is_not_free() {
        // Given
        let tx = happy_path_tx();
        {
            tx.world.lock().unwrap().mut_cell_unsafe(&v2(1, 1)).object =
                WorldObject::Crop { rotated: true };
        }

        // When
        let state = block_on(BuildCrops::new(tx, 0).process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_crops_if_no_traffic_entry() {
        // Given
        let mut state = happy_path_state();
        state.traffic = Traffic::new(3, 3, HashSet::new());

        // When
        let state = block_on(
            BuildCrops::new(happy_path_tx(), 0)
                .process(state, &Instruction::RefreshPositions(hashset! {v2(1, 1)})),
        );

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }

    #[test]
    fn should_not_build_crops_if_empty_traffic_entry() {
        // Given
        // Given
        let mut state = happy_path_state();
        state.traffic.set(&v2(1, 1), HashSet::new()).unwrap();

        // When
        let state = block_on(
            BuildCrops::new(happy_path_tx(), 0)
                .process(state, &Instruction::RefreshPositions(hashset! {v2(1, 1)})),
        );

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }
}
