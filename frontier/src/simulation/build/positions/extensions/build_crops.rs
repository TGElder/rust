use std::collections::{HashMap, HashSet};

use commons::grid::Grid;
use commons::rand::Rng;
use commons::V2;

use crate::build::{Build, BuildInstruction};
use crate::resource::Resource;
use crate::route::{RouteKey, RoutesExt};
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::traffic::Traffic;
use crate::traits::{InsertBuildInstruction, WithRoutes, WithTraffic, WithWorld};
use crate::world::{World, WorldObject};

impl<T> PositionBuildSimulation<T>
where
    T: InsertBuildInstruction + WithRoutes + WithWorld + WithTraffic,
{
    pub async fn build_crops(&mut self, positions: HashSet<V2<usize>>) {
        let crop_routes = self.get_crop_routes(positions).await;
        let crop_routes = self
            .filter_crop_routes_with_free_destination(crop_routes)
            .await;

        for (position, route_keys) in crop_routes {
            self.build_crops_at_position(position, route_keys).await;
        }
    }

    async fn get_crop_routes(
        &self,
        positions: HashSet<V2<usize>>,
    ) -> HashMap<V2<usize>, HashSet<RouteKey>> {
        self.tx
            .with_traffic(|traffic| get_crop_routes(&traffic, positions))
            .await
    }

    async fn filter_crop_routes_with_free_destination(
        &self,
        crop_routes: HashMap<V2<usize>, HashSet<RouteKey>>,
    ) -> HashMap<V2<usize>, HashSet<RouteKey>> {
        self.tx
            .with_world(|world| filter_crop_routes_with_free_destination(world, crop_routes))
            .await
    }

    async fn build_crops_at_position(
        &mut self,
        position: V2<usize>,
        route_keys: HashSet<RouteKey>,
    ) {
        self.tx
            .insert_build_instruction(BuildInstruction {
                what: Build::Crops {
                    position,
                    rotated: self.rng.gen(),
                },
                when: unwrap_or!(self.first_visit(route_keys).await, return),
            })
            .await;
    }

    async fn first_visit(&self, route_keys: HashSet<RouteKey>) -> Option<u128> {
        self.tx
            .with_routes(|routes| {
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
    traffic: &Traffic,
    positions: HashSet<V2<usize>>,
) -> HashMap<V2<usize>, HashSet<RouteKey>> {
    positions
        .into_iter()
        .flat_map(|position| get_crop_routes_for_position(traffic, position))
        .collect()
}

fn get_crop_routes_for_position(
    traffic: &Traffic,
    position: V2<usize>,
) -> Option<(V2<usize>, HashSet<RouteKey>)> {
    let route_keys = ok_or!(traffic.get(&position), return None);
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

    use commons::async_trait::async_trait;
    use commons::{v2, M};
    use futures::executor::block_on;

    use crate::route::{Route, RouteKey, Routes, RoutesExt};

    use super::*;

    struct Tx {
        build_instructions: Mutex<Vec<BuildInstruction>>,
        routes: Mutex<Routes>,
        traffic: Mutex<Traffic>,
        world: Mutex<World>,
    }

    #[async_trait]
    impl InsertBuildInstruction for Tx {
        async fn insert_build_instruction(&self, build_instruction: BuildInstruction) {
            self.build_instructions
                .lock()
                .unwrap()
                .push(build_instruction)
        }
    }

    #[async_trait]
    impl WithRoutes for Tx {
        async fn with_routes<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Routes) -> O + Send,
        {
            function(&self.routes.lock().unwrap())
        }

        async fn mut_routes<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Routes) -> O + Send,
        {
            function(&mut self.routes.lock().unwrap())
        }
    }

    #[async_trait]
    impl WithWorld for Tx {
        async fn with_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&World) -> O + Send,
        {
            function(&self.world.lock().unwrap())
        }

        async fn mut_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut World) -> O + Send,
        {
            function(&mut self.world.lock().unwrap())
        }
    }

    #[async_trait]
    impl WithTraffic for Tx {
        async fn with_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Traffic) -> O + Send,
        {
            function(&self.traffic.lock().unwrap())
        }

        async fn mut_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Traffic) -> O + Send,
        {
            function(&mut self.traffic.lock().unwrap())
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

        let mut traffic = Traffic::new(3, 3, HashSet::new());
        traffic
            .set(&v2(1, 1), hashset! {happy_path_route_key()})
            .unwrap();

        let world = World::new(M::from_element(3, 3, 1.0), 0.5);

        Tx {
            build_instructions: Mutex::default(),
            routes: Mutex::new(routes),
            traffic: Mutex::new(traffic),
            world: Mutex::new(world),
        }
    }

    #[test]
    fn should_build_crops_if_crop_route_ends_at_position() {
        // Given
        let mut build_crops = PositionBuildSimulation::new(happy_path_tx(), 0);

        // When
        block_on(build_crops.build_crops(hashset! {v2(1, 1)}));

        // Then
        assert!(matches!(
            build_crops.tx.build_instructions.lock().unwrap()[0],
            BuildInstruction{
                what: Build::Crops{position, .. },
                when: 11
            } if position == v2(1, 1)
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
        let mut sim = PositionBuildSimulation::new(tx, 0);

        // When
        block_on(sim.build_crops(hashset! {v2(1, 2)}));

        // Then
        assert!(sim.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_crops_if_crop_route_not_ending_at_position() {
        // Given
        let mut sim = PositionBuildSimulation::new(happy_path_tx(), 0);

        // When
        block_on(sim.build_crops(hashset! {v2(1, 0)}));

        // Then
        assert!(sim.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_crops_if_cell_is_not_free() {
        // Given
        let tx = happy_path_tx();
        {
            tx.world.lock().unwrap().mut_cell_unsafe(&v2(1, 1)).object =
                WorldObject::Crop { rotated: true };
        }
        let mut sim = PositionBuildSimulation::new(tx, 0);

        // When
        block_on(sim.build_crops(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_crops_if_no_traffic_entry() {
        // Given
        let tx = happy_path_tx();
        *tx.traffic.lock().unwrap() = Traffic::new(3, 3, HashSet::new());

        let mut sim = PositionBuildSimulation::new(tx, 0);

        // When
        block_on(sim.build_crops(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_crops_if_empty_traffic_entry() {
        // Given
        let tx = happy_path_tx();
        tx.traffic
            .lock()
            .unwrap()
            .set(&v2(1, 1), HashSet::new())
            .unwrap();

        let mut sim = PositionBuildSimulation::new(tx, 0);

        // When
        block_on(sim.build_crops(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.tx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_crops_if_invalid_route() {
        // Given
        let mut tx = happy_path_tx();
        tx.routes = Mutex::default();

        let mut sim = PositionBuildSimulation::new(tx, 0);

        // When
        block_on(sim.build_crops(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.tx.build_instructions.lock().unwrap().is_empty());
    }
}
