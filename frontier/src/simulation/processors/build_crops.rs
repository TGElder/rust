use std::collections::{HashMap, HashSet};

use commons::grid::Grid;
use commons::rand::prelude::SmallRng;
use commons::rand::{Rng, SeedableRng};

use crate::resource::Resource;
use crate::route::{RouteKey, RoutesExt};
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
        let positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };

        let crop_routes = get_crop_routes(&state, positions);
        let crop_routes = self
            .filter_crop_routes_with_free_destination(crop_routes)
            .await;

        for (position, route_keys) in crop_routes {
            self.process_position(&mut state, position, route_keys)
                .await;
        }

        state
    }
}

impl<T> BuildCrops<T>
where
    T: SendRoutes + SendWorld,
{
    pub fn new(tx: T, seed: u64) -> BuildCrops<T> {
        BuildCrops {
            tx,
            rng: SeedableRng::seed_from_u64(seed),
        }
    }

    async fn filter_crop_routes_with_free_destination(
        &self,
        crop_routes: HashMap<V2<usize>, HashSet<RouteKey>>,
    ) -> HashMap<V2<usize>, HashSet<RouteKey>> {
        self.tx
            .send_world(move |world| filter_crop_routes_with_free_destination(world, crop_routes))
            .await
    }

    async fn process_position(
        &mut self,
        state: &mut State,
        position: V2<usize>,
        route_keys: HashSet<RouteKey>,
    ) {
        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position,
                rotated: self.rng.gen(),
            },
            when: unwrap_or!(self.first_visit(route_keys).await, return),
        });
    }

    async fn first_visit(&self, route_keys: HashSet<RouteKey>) -> Option<u128> {
        self.tx
            .send_routes(move |routes| {
                route_keys
                    .into_iter()
                    .flat_map(|route_key| routes.get_route(&route_key))
                    .map(|route| route.start_micros + route.duration.as_micros())
                    .min()
            })
            .await
    }
}

fn get_crop_routes(
    state: &State,
    positions: HashSet<V2<usize>>,
) -> HashMap<V2<usize>, HashSet<RouteKey>> {
    positions
        .into_iter()
        .flat_map(|position| get_crop_routes_for_position(state, position))
        .collect()
}

fn get_crop_routes_for_position(
    state: &State,
    position: V2<usize>,
) -> Option<(V2<usize>, HashSet<RouteKey>)> {
    let route_keys = ok_or!(state.traffic.get(&position), return None);
    let crop_route_keys = route_keys
        .iter()
        .filter(|route_key| {
            route_key.resource == Resource::Crops && route_key.destination == position
        })
        .cloned()
        .collect();
    Some((position, crop_route_keys))
}

fn filter_crop_routes_with_free_destination(
    world: &World,
    crop_routes: HashMap<V2<usize>, HashSet<RouteKey>>,
) -> HashMap<V2<usize>, HashSet<RouteKey>> {
    crop_routes
        .into_iter()
        .map(|(position, route_key)| (position, free_destinations(world, route_key)))
        .collect()
}

fn free_destinations(world: &World, route_keys: HashSet<RouteKey>) -> HashSet<RouteKey> {
    route_keys
        .into_iter()
        .filter(|route_key| is_free(world, &route_key.destination))
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

    use crate::route::{Route, RouteKey, Routes, RoutesExt};

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

    #[test]
    fn should_not_build_crops_if_invalid_route() {
        // Given
        let mut tx = happy_path_tx();
        tx.routes = Arm::default();

        // When
        let state = block_on(BuildCrops::new(tx, 0).process(
            happy_path_state(),
            &Instruction::RefreshPositions(hashset! {v2(1, 1)}),
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
    }
}
