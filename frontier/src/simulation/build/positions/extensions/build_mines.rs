use std::collections::{HashMap, HashSet};

use commons::grid::Grid;
use commons::V2;

use crate::build::{Build, BuildInstruction};
use crate::resource::{Mine, MineRule};
use crate::route::RouteKey;
use crate::simulation::build::positions::PositionBuildSimulation;
use crate::traffic::Traffic;
use crate::traits::has::HasParameters;
use crate::traits::{InsertBuildInstruction, Micros, WithTraffic, WithWorld};

impl<T> PositionBuildSimulation<T>
where
    T: HasParameters + InsertBuildInstruction + Micros + WithTraffic + WithWorld,
{
    pub async fn build_mines(&self, positions: HashSet<V2<usize>>) {
        let expected = self.get_expected_mines(positions).await;
        let changes = self.get_changes(expected).await;
        self.apply_changes(changes).await;
    }

    async fn get_expected_mines(&self, positions: HashSet<V2<usize>>) -> HashMap<V2<usize>, Mine> {
        let rules = &self.cx.parameters().mine_rules;
        self.cx
            .with_traffic(|traffic| {
                positions
                    .into_iter()
                    .map(|position| (position, get_expected_mine(&position, traffic, rules)))
                    .collect()
            })
            .await
    }

    async fn get_changes(&self, expected: HashMap<V2<usize>, Mine>) -> HashMap<V2<usize>, Mine> {
        self.cx
            .with_world(|world| {
                expected
                    .into_iter()
                    .filter(|(position, mine)| {
                        !mine.matches(&world.get_cell_unsafe(position).object)
                    })
                    .collect()
            })
            .await
    }

    async fn apply_changes(&self, changes: HashMap<V2<usize>, Mine>) {
        let when = self.cx.micros().await;
        for (position, mine) in changes {
            self.cx
                .insert_build_instruction(BuildInstruction {
                    what: Build::Mine { position, mine },
                    when,
                })
                .await
        }
    }
}

fn get_expected_mine(position: &V2<usize>, traffic: &Traffic, rules: &[MineRule]) -> Mine {
    let traffic = traffic.get_cell_unsafe(position);
    for rule in rules {
        if traffic
            .iter()
            .filter(|RouteKey { destination, .. }| position == destination)
            .any(|RouteKey { resource, .. }| *resource == rule.resource)
        {
            return rule.mine;
        }
    }
    Mine::None
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use std::sync::Mutex;

    use commons::async_trait::async_trait;
    use commons::{v2, M};

    use crate::parameters::Parameters;
    use crate::resource::Resource;
    use crate::world::{VegetationType, World, WorldObject};

    use super::*;

    struct Cx {
        build_instructions: Mutex<Vec<BuildInstruction>>,
        micros: u128,
        parameters: Parameters,
        traffic: Traffic,
        world: World,
    }

    impl HasParameters for Cx {
        fn parameters(&self) -> &Parameters {
            &self.parameters
        }
    }

    #[async_trait]
    impl InsertBuildInstruction for Cx {
        async fn insert_build_instruction(&self, build_instruction: BuildInstruction) {
            self.build_instructions
                .lock()
                .unwrap()
                .push(build_instruction);
        }
    }

    #[async_trait]
    impl Micros for Cx {
        async fn micros(&self) -> u128 {
            self.micros
        }
    }

    #[async_trait]
    impl WithTraffic for Cx {
        async fn with_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Traffic) -> O + Send,
        {
            function(&self.traffic)
        }

        async fn mut_traffic<F, O>(&self, _: F) -> O
        where
            F: FnOnce(&mut Traffic) -> O + Send,
        {
            panic!("Not expecting traffic to be mutated");
        }
    }

    #[async_trait]
    impl WithWorld for Cx {
        async fn with_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&World) -> O + Send,
        {
            function(&self.world)
        }

        async fn mut_world<F, O>(&self, _: F) -> O
        where
            F: FnOnce(&mut World) -> O + Send,
        {
            panic!("Not expecting world to be mutated")
        }
    }

    impl Default for Cx {
        fn default() -> Self {
            let position = v2(1, 2);
            let route_key = RouteKey {
                settlement: v2(0, 0),
                destination: position,
                resource: Resource::Crops,
            };
            let mut traffic = Traffic::new(3, 3, HashSet::with_capacity(0));
            traffic.mut_cell_unsafe(&position).insert(route_key);

            Cx {
                build_instructions: Mutex::default(),
                micros: 808,
                parameters: Parameters {
                    mine_rules: vec![
                        MineRule {
                            resource: Resource::Pasture,
                            mine: Mine::Pasture,
                        },
                        MineRule {
                            resource: Resource::Crops,
                            mine: Mine::Crop,
                        },
                    ],
                    ..Parameters::default()
                },
                traffic,
                world: World::new(M::zeros(3, 3), 0.0),
            }
        }
    }

    #[test]
    fn should_build_mine_if_mine_expected_and_mine_does_not_exist() {
        // Given
        let sim = PositionBuildSimulation::new(Cx::default());

        // When
        block_on(sim.build_mines(hashset! {v2(1, 2)}));

        // Then
        assert_eq!(
            *sim.cx.build_instructions.lock().unwrap(),
            vec![BuildInstruction {
                what: Build::Mine {
                    position: v2(1, 2),
                    mine: Mine::Crop,
                },
                when: 808,
            }]
        );
    }

    #[test]
    fn should_build_mine_if_object_already_exists_at_position() {
        // Given
        let mut cx = Cx::default();
        cx.world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Vegetation {
            vegetation_type: VegetationType::Cactus,
            offset: v2(0.0, 0.0),
        };

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_mines(hashset! {v2(1, 2)}));

        // Then
        assert_eq!(
            *sim.cx.build_instructions.lock().unwrap(),
            vec![BuildInstruction {
                what: Build::Mine {
                    position: v2(1, 2),
                    mine: Mine::Crop,
                },
                when: 808,
            }]
        );
    }

    #[test]
    fn should_not_build_mine_if_mine_expected_and_mine_already_exists() {
        // Given
        let mut cx = Cx::default();
        cx.world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Crop { rotated: true };

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_mines(hashset! {v2(1, 2)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_mine_if_destination_not_at_position() {
        // Given
        let mut cx = Cx::default();
        *cx.traffic.mut_cell_unsafe(&v2(1, 2)) = hashset! {
           RouteKey {
               settlement: v2(0, 0),
               destination: v2(2, 2),
               resource: Resource::Crops,
           }
        };

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_mines(hashset! {v2(1, 2)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_build_mine_if_resource_has_no_mine() {
        // Given
        let mut cx = Cx::default();
        *cx.traffic.mut_cell_unsafe(&v2(1, 2)) = hashset! {
           RouteKey {
               settlement: v2(0, 0),
               destination: v2(1, 2),
               resource: Resource::Wood,
           }
        };

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_mines(hashset! {v2(1, 2)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn should_build_first_matching_mine() {
        // Given
        let mut cx = Cx::default();
        cx.traffic.mut_cell_unsafe(&v2(1, 2)).insert(RouteKey {
            settlement: v2(0, 0),
            destination: v2(1, 2),
            resource: Resource::Pasture,
        });

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_mines(hashset! {v2(1, 2)}));

        // Then
        assert_eq!(
            *sim.cx.build_instructions.lock().unwrap(),
            vec![BuildInstruction {
                what: Build::Mine {
                    position: v2(1, 2),
                    mine: Mine::Pasture,
                },
                when: 808,
            }]
        );
    }

    #[test]
    fn should_remove_mine_if_no_mine_expected_and_mine_exists() {
        // Given
        let mut cx = Cx::default();
        *cx.traffic.mut_cell_unsafe(&v2(1, 2)) = hashset! {};
        cx.world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Crop { rotated: true };

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_mines(hashset! {v2(1, 2)}));

        // Then
        assert_eq!(
            *sim.cx.build_instructions.lock().unwrap(),
            vec![BuildInstruction {
                what: Build::Mine {
                    position: v2(1, 2),
                    mine: Mine::None,
                },
                when: 808,
            }]
        );
    }

    #[test]
    fn should_not_remove_mine_if_no_mine_expected_and_mine_does_not_exist() {
        // Given
        let mut cx = Cx::default();
        *cx.traffic.mut_cell_unsafe(&v2(1, 2)) = hashset! {};

        let sim = PositionBuildSimulation::new(cx);

        // When
        block_on(sim.build_mines(hashset! {v2(1, 2)}));

        // Then
        assert!(sim.cx.build_instructions.lock().unwrap().is_empty());
    }
}
