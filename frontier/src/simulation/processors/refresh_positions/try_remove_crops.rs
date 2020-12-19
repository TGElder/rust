use super::*;

use crate::game::traits::HasWorld;
use crate::resource::Resource;
use crate::traits::RemoveWorldObject;
use crate::world::{WorldCell, WorldObject};
use commons::grid::Grid;
use futures::executor::ThreadPool;

const FARM_RESOURCE: Resource = Resource::Crops;

pub fn try_remove_crops<G, X>(
    state: &mut State,
    game: &mut G,
    x: &X,
    pool: &ThreadPool,
    traffic: &PositionTrafficSummary,
) where
    G: HasWorld,
    X: RemoveWorldObject + Clone + Send + Sync + 'static,
{
    let crop_routes = get_crop_routes(&traffic);
    if !crop_routes.is_empty() {
        return;
    }

    state.build_queue.remove(&BuildKey::Crops(traffic.position));

    if cell_has_crops(game, &traffic.position) {
        remove_crops(x, pool, traffic.position);
    }
}

fn get_crop_routes(traffic: &PositionTrafficSummary) -> Vec<&RouteSummary> {
    traffic
        .routes
        .iter()
        .filter(|route| route.resource == FARM_RESOURCE && route.destination == traffic.position)
        .collect()
}

fn cell_has_crops(world: &dyn HasWorld, position: &V2<usize>) -> bool {
    if let Some(WorldCell {
        object: WorldObject::Crop { .. },
        ..
    }) = world.world().get_cell(&position)
    {
        true
    } else {
        false
    }
}

fn remove_crops<X>(x: &X, pool: &ThreadPool, position: V2<usize>)
where
    X: RemoveWorldObject + Clone + Send + Sync + 'static,
{
    let x = x.clone();
    pool.spawn_ok(async move { x.remove_world_object(position).await });
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::world::World;
    use commons::grid::Grid;
    use commons::{v2, Arm, M};
    use std::collections::HashSet;
    use std::time::Duration;

    fn world() -> World {
        World::new(M::zeros(3, 3), 0.0)
    }

    #[derive(Clone, Default)]
    struct X {
        removed_objects: Arm<HashSet<V2<usize>>>,
    }

    #[async_trait]
    impl RemoveWorldObject for X {
        async fn remove_world_object(&self, position: V2<usize>) {
            self.removed_objects.lock().unwrap().insert(position);
        }
    }

    #[test]
    fn should_remove_crops_if_no_routes_here() {
        // Given
        let traffic = PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![],
            adjacent: vec![],
        };

        let mut state = State::default();
        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: v2(1, 2),
                rotated: true,
            },
            when: 10,
        });

        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Crop { rotated: true };
        let x = X::default();

        // When
        try_remove_crops(
            &mut state,
            &mut world,
            &x,
            &ThreadPool::new().unwrap(),
            &traffic,
        );
        while x.removed_objects.lock().unwrap().is_empty() {}

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
        assert_eq!(
            *x.removed_objects.lock().unwrap(),
            hashset! {traffic.position}
        );
    }

    #[test]
    fn should_remove_crops_if_route_for_crops_does_not_end_here() {
        // Given
        let traffic = PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 1,
                origin: v2(0, 1),
                destination: v2(1, 3),
                nation: String::default(),
                first_visit: 0,
                duration: Duration::default(),
                resource: Resource::Crops,
                ports: hashset! {},
            }],
            adjacent: vec![],
        };

        let mut state = State::default();
        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: v2(1, 2),
                rotated: true,
            },
            when: 10,
        });

        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Crop { rotated: true };
        let x = X::default();

        // When
        try_remove_crops(
            &mut state,
            &mut world,
            &x,
            &ThreadPool::new().unwrap(),
            &traffic,
        );
        while x.removed_objects.lock().unwrap().is_empty() {}

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
        assert_eq!(
            *x.removed_objects.lock().unwrap(),
            hashset! {traffic.position}
        );
    }

    #[test]
    fn should_remove_crops_if_route_not_for_crops_ends_here() {
        // Given
        let traffic = PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 1,
                origin: v2(0, 1),
                destination: v2(1, 2),
                nation: String::default(),
                first_visit: 0,
                duration: Duration::default(),
                resource: Resource::Pasture,
                ports: hashset! {},
            }],
            adjacent: vec![],
        };

        let mut state = State::default();
        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: v2(1, 2),
                rotated: true,
            },
            when: 10,
        });

        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Crop { rotated: true };
        let x = X::default();

        // When
        try_remove_crops(
            &mut state,
            &mut world,
            &x,
            &ThreadPool::new().unwrap(),
            &traffic,
        );
        while x.removed_objects.lock().unwrap().is_empty() {}

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
        assert_eq!(
            *x.removed_objects.lock().unwrap(),
            hashset! {traffic.position}
        );
    }

    #[test]
    fn should_do_nothing_if_route_for_crops_end_here() {
        // Given
        let traffic = PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 1,
                origin: v2(0, 1),
                destination: v2(1, 2),
                nation: String::default(),
                first_visit: 0,
                duration: Duration::default(),
                resource: Resource::Crops,
                ports: hashset! {},
            }],
            adjacent: vec![],
        };

        let mut build_queue = BuildQueue::default();
        build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: v2(1, 2),
                rotated: true,
            },
            when: 10,
        });
        let mut state = State::default();
        state.build_queue = build_queue.clone();

        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Crop { rotated: true };
        let x = X::default();

        // When
        try_remove_crops(
            &mut state,
            &mut world,
            &x,
            &ThreadPool::new().unwrap(),
            &traffic,
        );

        // Then
        assert_eq!(state.build_queue, build_queue);
        assert_eq!(*x.removed_objects.lock().unwrap(), hashset! {});
    }

    #[test]
    fn should_not_remove_crops_if_cell_does_not_have_crops() {
        // Given
        let traffic = PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![],
            adjacent: vec![],
        };

        let mut state = State::default();
        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: v2(1, 2),
                rotated: true,
            },
            when: 10,
        });

        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Crop { rotated: true };
        let x = X::default();

        // When
        try_remove_crops(
            &mut state,
            &mut world,
            &x,
            &ThreadPool::new().unwrap(),
            &traffic,
        );

        // Then
        assert_eq!(state.build_queue, BuildQueue::default()); // Should still remove build instruction
        assert_eq!(*x.removed_objects.lock().unwrap(), hashset! {});
    }
}
