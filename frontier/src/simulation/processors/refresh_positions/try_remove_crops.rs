use super::*;

use crate::game::traits::HasWorld;
use crate::resource::Resource;
use crate::traits::SetWorldObject;
use crate::world::{WorldCell, WorldObject};
use commons::executor::ThreadPool;
use commons::grid::Grid;

const FARM_RESOURCE: Resource = Resource::Crops;

pub fn try_remove_crops<G, X>(
    state: &mut State,
    game: &mut G,
    x: &X,
    pool: &ThreadPool,
    traffic: &PositionTrafficSummary,
) where
    G: HasWorld,
    X: SetWorldObject + Clone + Send + Sync + 'static,
{
    let crop_routes = get_crop_routes(&traffic);
    if !crop_routes.is_empty() {
        return;
    }

    state.build_queue.remove(&BuildKey::Crops(traffic.position));

    if cell_has_crops(game, &traffic.position) {
        let x = x.clone();
        let position = traffic.position;
        pool.spawn_ok(async move { x.force_world_object(WorldObject::None, position).await })
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
    struct MockX {
        removed_objects: Arm<HashSet<V2<usize>>>,
    }

    #[async_trait]
    impl SetWorldObject for MockX {
        async fn set_world_object(&self, _: WorldObject, _: V2<usize>) -> bool {
            todo!()
        }

        async fn force_world_object(&self, _: WorldObject, position: V2<usize>) {
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
        let x = MockX::default();

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
        let x = MockX::default();

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
        let x = MockX::default();

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
        let x = MockX::default();

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
        let x = MockX::default();

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
