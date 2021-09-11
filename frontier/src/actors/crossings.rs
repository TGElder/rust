use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::{Rotation, Vehicle};
use crate::bridges::{Bridge, BridgeType, InvalidBridge, Pier};
use crate::traits::has::HasParameters;
use crate::traits::{WithBridges, WithWorld};
use crate::world::World;

pub struct Crossings<T> {
    cx: T,
}

impl<T> Crossings<T>
where
    T: HasParameters + WithBridges + WithWorld,
{
    pub fn new(cx: T) -> Crossings<T> {
        Crossings { cx }
    }

    pub async fn new_game(&self) {
        let crossings = self.get_crossings().await;
        let bridges = self.get_bridges(crossings).await;
        self.build_bridges(bridges).await;
    }

    async fn get_crossings(&self) -> Vec<[Pier; 3]> {
        let min_navigable_river_width = self.cx.parameters().npc_travel.min_navigable_river_width;
        let max_gradient = self.cx.parameters().npc_travel.max_walk_gradient;

        self.cx
            .with_world(|world| get_crossings(world, &min_navigable_river_width, &max_gradient))
            .await
    }

    async fn get_bridges(&self, crossings: Vec<[Pier; 3]>) -> Vec<Bridge> {
        crossings.into_iter().flat_map(get_bridge).collect()
    }

    async fn build_bridges(&self, to_build: Vec<Bridge>) {
        self.cx
            .mut_bridges(|bridges| {
                for bridge in to_build {
                    bridges
                        .entry(bridge.total_edge())
                        .or_default()
                        .insert(bridge);
                }
            })
            .await;
    }
}

fn get_crossings(
    world: &World,
    min_navigable_river_width: &f32,
    max_gradient: &f32,
) -> Vec<[Pier; 3]> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = v2(x, y);

            if let (Some(left), Some(right)) = (
                world.offset(&position, v2(-1, 0)),
                world.offset(&position, v2(1, 0)),
            ) {
                let horizontal = [left, position, right];
                if let Some(crossing) =
                    is_crossing(world, min_navigable_river_width, max_gradient, &horizontal)
                {
                    out.push(crossing);
                }
            }

            if let (Some(down), Some(up)) = (
                world.offset(&position, v2(0, -1)),
                world.offset(&position, v2(0, 1)),
            ) {
                let vertical = [down, position, up];
                if let Some(crossing) =
                    is_crossing(world, min_navigable_river_width, max_gradient, &vertical)
                {
                    out.push(crossing);
                }
            }
        }
    }
    out
}

fn is_crossing(
    world: &World,
    min_navigable_river_width: &f32,
    max_gradient: &f32,
    positions: &[V2<usize>; 3],
) -> Option<[Pier; 3]> {
    if world.is_sea(&positions[0]) || world.is_sea(&positions[2]) {
        return None;
    }

    let cells = positions
        .iter()
        .flat_map(|position| world.get_cell(position))
        .collect::<Vec<_>>();

    if cells.len() != 3 {
        // At least one of the positions is out of bounds
        return None;
    }

    if cells[1].river.longest_side() < *min_navigable_river_width {
        return None;
    }

    if cells[0].elevation <= cells[1].elevation || cells[2].elevation <= cells[1].elevation {
        // Bridge is convex, meaning it will pass beneath terrain
        return None;
    }

    if cells[0].river.here() || cells[2].river.here() {
        return None;
    }

    if world.get_rise(&positions[0], &positions[1]).unwrap().abs() > *max_gradient {
        return None;
    }

    if world.get_rise(&positions[1], &positions[2]).unwrap().abs() > *max_gradient {
        return None;
    }

    let rotation = Rotation::from_positions(&positions[0], &positions[2]).ok()?;
    Some([
        Pier {
            position: positions[0],
            elevation: world.get_cell_unsafe(&positions[0]).elevation,
            platform: true,
            rotation,
            vehicle: Vehicle::None,
        },
        Pier {
            position: positions[1],
            elevation: world.get_cell_unsafe(&positions[1]).elevation,
            platform: false,
            rotation,
            vehicle: Vehicle::None,
        },
        Pier {
            position: positions[2],
            elevation: world.get_cell_unsafe(&positions[2]).elevation,
            platform: true,
            rotation,
            vehicle: Vehicle::None,
        },
    ])
}

fn get_bridge(crossing: [Pier; 3]) -> Result<Bridge, InvalidBridge> {
    Bridge {
        piers: crossing.to_vec(),
        bridge_type: BridgeType::Theoretical,
    }
    .validate()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use commons::async_trait::async_trait;
    use commons::edge::Edge;
    use commons::junction::PositionJunction;
    use commons::{v2, M};
    use futures::executor::block_on;

    use crate::avatar::AvatarTravelParams;
    use crate::bridges::Bridges;
    use crate::parameters::Parameters;

    use super::*;

    struct Cx {
        parameters: Parameters,
        bridges: Mutex<Bridges>,
        world: Mutex<World>,
    }

    impl HasParameters for Cx {
        fn parameters(&self) -> &Parameters {
            &self.parameters
        }
    }

    #[async_trait]
    impl WithBridges for Cx {
        async fn with_bridges<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Bridges) -> O + Send,
        {
            function(&self.bridges.lock().unwrap())
        }

        async fn mut_bridges<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Bridges) -> O + Send,
        {
            function(&mut self.bridges.lock().unwrap())
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

    fn cx(world: World) -> Cx {
        Cx {
            parameters: Parameters {
                npc_travel: AvatarTravelParams {
                    min_navigable_river_width: 0.5,
                    max_navigable_river_gradient: 0.5,
                    ..AvatarTravelParams::default()
                },
                ..Parameters::default()
            },
            bridges: Mutex::default(),
            world: Mutex::new(world),
        }
    }

    #[test]
    fn should_add_crossing_over_horizontal_river() {
        // Given
        let mut world = World::new(M::from_vec(3, 1, vec![1.0, 0.9, 1.0]), 0.5);

        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let crossings = Crossings::new(cx);

        // When
        block_on(crossings.new_game());

        // Then
        assert_eq!(
            *crossings.cx.bridges.lock().unwrap(),
            hashmap! {
                Edge::new(v2(0, 0), v2(2, 0)) => hashset!{
                    Bridge{
                        piers: vec![
                            Pier{
                                position: v2(0, 0),
                                elevation: 1.0,
                                platform: true,
                                rotation: Rotation::Right,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(1, 0),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Right,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(2, 0),
                                elevation: 1.0,
                                platform: true,
                                rotation: Rotation::Right,
                                vehicle: Vehicle::None,
                            },
                        ],
                        bridge_type: BridgeType::Theoretical
                    }
                }
            }
        );
    }

    #[test]
    fn should_add_crossing_over_vertical_river() {
        // Given
        let mut world = World::new(M::from_vec(1, 3, vec![1.0, 0.9, 1.0]), 0.5);

        let mut river_1 = PositionJunction::new(v2(0, 1));
        river_1.junction.vertical.width = 1.0;
        river_1.junction.vertical.from = true;
        river_1.junction.vertical.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let crossings = Crossings::new(cx);

        // When
        block_on(crossings.new_game());

        // Then
        assert_eq!(
            *crossings.cx.bridges.lock().unwrap(),
            hashmap! {
                Edge::new(v2(0, 0), v2(0, 2)) => hashset!{
                    Bridge{
                        piers: vec![
                            Pier{
                                position: v2(0, 0),
                                elevation: 1.0,
                                platform: true,
                                rotation: Rotation::Up,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(0, 1),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Up,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(0, 2),
                                elevation: 1.0,
                                platform: true,
                                rotation: Rotation::Up,
                                vehicle: Vehicle::None,
                            },
                        ],
                        bridge_type: BridgeType::Theoretical
                    }
                }
            }
        );
    }

    #[test]
    fn should_not_add_crossing_along_river() {
        // Given
        let mut world = World::new(M::from_vec(3, 1, vec![1.0, 0.9, 1.0]), 0.5);

        let mut river_1 = PositionJunction::new(v2(0, 0));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let mut river_2 = PositionJunction::new(v2(1, 0));
        river_2.junction.horizontal.width = 1.0;
        river_2.junction.horizontal.from = true;
        river_2.junction.horizontal.to = true;
        world.add_river(river_2);

        let mut river_3 = PositionJunction::new(v2(2, 0));
        river_3.junction.horizontal.width = 1.0;
        river_3.junction.horizontal.from = true;
        river_3.junction.horizontal.to = true;
        world.add_river(river_3);

        let cx = cx(world);

        let crossings = Crossings::new(cx);

        // When
        block_on(crossings.new_game());

        // Then
        assert_eq!(*crossings.cx.bridges.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_not_add_crossing_over_stream() {
        // Given
        let mut world = World::new(M::from_vec(3, 1, vec![1.0, 0.9, 1.0]), 0.5);

        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.horizontal.width = 0.1;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let crossings = Crossings::new(cx);

        // When
        block_on(crossings.new_game());

        // Then
        assert_eq!(*crossings.cx.bridges.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_not_add_convex_crossing() {
        // Given
        let mut world = World::new(M::from_vec(3, 1, vec![0.9, 1.0, 0.9]), 0.5);

        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let crossings = Crossings::new(cx);

        // When
        block_on(crossings.new_game());

        // Then
        assert_eq!(*crossings.cx.bridges.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_not_add_crossing_exceeding_max_gradient() {
        // Given
        let mut world = World::new(M::from_vec(3, 1, vec![1.0, 0.1, 1.0]), 0.5);

        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let crossings = Crossings::new(cx);

        // When
        block_on(crossings.new_game());

        // Then
        assert_eq!(*crossings.cx.bridges.lock().unwrap(), hashmap! {});
    }
}
