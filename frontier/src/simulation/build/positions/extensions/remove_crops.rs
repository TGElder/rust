use std::collections::HashSet;

use commons::grid::Grid;
use commons::V2;

use crate::build::BuildKey;
use crate::resource::Resource;
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::traffic::Traffic;
use crate::traits::{
    GetBuildInstruction, RefreshTargets, RemoveBuildInstruction, RemoveWorldObject, WithTraffic,
    WithWorld,
};
use crate::world::{World, WorldCell, WorldObject};

impl<T> PositionBuildSimulation<T>
where
    T: GetBuildInstruction
        + RefreshTargets
        + RemoveBuildInstruction
        + RemoveWorldObject
        + WithTraffic
        + WithWorld,
{
    pub async fn remove_crops(&self, mut positions: HashSet<V2<usize>>) {
        self.filter_without_crop_routes(&mut positions).await;

        for position in positions.iter() {
            if self.has_crops_build_instruction(position).await {
                self.cx
                    .remove_build_instruction(&BuildKey::Object(*position))
                    .await;
            }
        }

        for position in self.have_crops(positions.clone()).await {
            self.cx.remove_world_object(&position).await;
        }

        self.cx.refresh_targets(positions).await;
    }

    async fn filter_without_crop_routes(&self, positions: &mut HashSet<V2<usize>>) {
        self.cx
            .with_traffic(move |traffic| {
                positions.retain(|position| !has_crop_routes(&traffic, position))
            })
            .await
    }

    async fn has_crops_build_instruction(&self, position: &V2<usize>) -> bool {
        self.cx
            .get_build_instruction(&BuildKey::Object(*position))
            .await
            .is_some()
    }

    async fn have_crops(&self, positions: HashSet<V2<usize>>) -> Vec<V2<usize>> {
        self.cx
            .with_world(|world| have_crops(world, positions))
            .await
    }
}

fn has_crop_routes(traffic: &Traffic, position: &V2<usize>) -> bool {
    ok_or!(traffic.get(&position), return false)
        .iter()
        .any(|route| route.resource == Resource::Crops && route.destination == *position)
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

    use commons::async_trait::async_trait;
    use commons::{v2, M};
    use futures::executor::block_on;

    use crate::build::{Build, BuildInstruction};
    use crate::route::RouteKey;

    use super::*;

    struct Cx {
        get_build_instruction: Option<BuildInstruction>,
        refreshed_targets: Mutex<HashSet<V2<usize>>>,
        removed_build_instructions: Mutex<HashSet<BuildKey>>,
        removed_world_objects: Mutex<Vec<V2<usize>>>,
        traffic: Mutex<Traffic>,
        world: Mutex<World>,
    }

    #[async_trait]
    impl GetBuildInstruction for Cx {
        async fn get_build_instruction(&self, _: &BuildKey) -> Option<BuildInstruction> {
            self.get_build_instruction.to_owned()
        }
    }

    #[async_trait]
    impl RefreshTargets for Cx {
        async fn refresh_targets(&self, positions: HashSet<V2<usize>>) {
            self.refreshed_targets.lock().unwrap().extend(positions);
        }
    }

    #[async_trait]
    impl RemoveBuildInstruction for Cx {
        async fn remove_build_instruction(&self, build_key: &BuildKey) {
            self.removed_build_instructions
                .lock()
                .unwrap()
                .insert(*build_key);
        }
    }

    #[async_trait]
    impl RemoveWorldObject for Cx {
        async fn remove_world_object(&self, position: &V2<usize>) {
            self.removed_world_objects.lock().unwrap().push(*position);
        }
    }

    #[async_trait]
    impl WithWorld for Cx {
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
    impl WithTraffic for Cx {
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

    fn happy_path_tx() -> Cx {
        let mut world = World::new(M::zeros(3, 3), 0.0);
        world.mut_cell_unsafe(&v2(1, 1)).object = WorldObject::Crop { rotated: true };
        Cx {
            get_build_instruction: None,
            refreshed_targets: Mutex::default(),
            removed_build_instructions: Mutex::default(),
            removed_world_objects: Mutex::default(),
            traffic: Mutex::new(Traffic::same_size_as(&world, hashset! {})),
            world: Mutex::new(world),
        }
    }

    #[test]
    fn should_remove_crops_if_no_traffic() {
        // Given
        let cx = happy_path_tx();
        let sim = PositionBuildSimulation::new(cx, 0);

        // When
        block_on(sim.remove_crops(hashset! {v2(1, 1)}));

        // Then
        assert_eq!(
            *sim.cx.removed_world_objects.lock().unwrap(),
            vec![v2(1, 1)]
        );
        assert_eq!(
            *sim.cx.refreshed_targets.lock().unwrap(),
            hashset! {v2(1, 1)}
        );
    }

    #[test]
    fn should_remove_crops_if_non_crop_traffic() {
        // Given
        let cx = happy_path_tx();
        cx.traffic
            .lock()
            .unwrap()
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

        let sim = PositionBuildSimulation::new(cx, 0);

        // When
        block_on(sim.remove_crops(hashset! {v2(1, 1)}));

        // Then
        assert_eq!(
            *sim.cx.removed_world_objects.lock().unwrap(),
            vec![v2(1, 1)]
        );
        assert_eq!(
            *sim.cx.refreshed_targets.lock().unwrap(),
            hashset! {v2(1, 1)}
        );
    }

    #[test]
    fn should_remove_instruction_from_build_queue() {
        // Given
        let mut cx = happy_path_tx();
        cx.get_build_instruction = Some(BuildInstruction {
            what: Build::Object {
                position: v2(1, 1),
                object: WorldObject::Crop { rotated: true },
            },
            when: 1,
        });
        cx.world.lock().unwrap().mut_cell_unsafe(&v2(1, 1)).object = WorldObject::None;

        let sim = PositionBuildSimulation::new(cx, 0);

        // When
        block_on(sim.remove_crops(hashset! {v2(1, 1)}));

        // Then
        assert_eq!(
            *sim.cx.removed_build_instructions.lock().unwrap(),
            hashset! { BuildKey::Object(v2(1, 1)) }
        );
    }

    #[test]
    fn should_not_remove_crops_if_crop_traffic() {
        // Given
        let cx = happy_path_tx();
        cx.traffic
            .lock()
            .unwrap()
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

        let sim = PositionBuildSimulation::new(cx, 0);

        // When
        block_on(sim.remove_crops(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.removed_world_objects.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_remove_build_instruction_if_crop_traffic() {
        // Given
        let mut cx = happy_path_tx();
        cx.get_build_instruction = Some(BuildInstruction {
            what: Build::Object {
                position: v2(1, 1),
                object: WorldObject::Crop { rotated: true },
            },
            when: 1,
        });
        cx.traffic
            .lock()
            .unwrap()
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
        cx.world.lock().unwrap().mut_cell_unsafe(&v2(1, 1)).object = WorldObject::None;

        let sim = PositionBuildSimulation::new(cx, 0);

        // When
        block_on(sim.remove_crops(hashset! {v2(1, 1)}));

        // Then
        assert!(sim.cx.removed_build_instructions.lock().unwrap().is_empty());
        assert!(sim.cx.refreshed_targets.lock().unwrap().is_empty());
    }
}
