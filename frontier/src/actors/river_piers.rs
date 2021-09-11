use commons::grid::Grid;
use commons::{v2, V2};

use crate::avatar::{Rotation, Vehicle};
use crate::bridges::{Bridge, BridgeType, Pier};
use crate::traits::{WithBridges, WithWorld};
use crate::world::World;

pub struct RiverPiers<T> {
    cx: T,
    parameters: RiverPierParameters,
}

pub struct RiverPierParameters {
    pub min_navigable_river_width: f32,
    pub max_landing_zone_gradient: f32,
    pub max_gradient: f32,
}

impl<T> RiverPiers<T>
where
    T: WithBridges + WithWorld + Sync,
{
    pub fn new(cx: T, parameters: RiverPierParameters) -> RiverPiers<T> {
        RiverPiers { cx, parameters }
    }

    pub async fn new_game(&self) {
        let piers = self.get_piers().await;
        let bridges = self.get_bridges(piers).await;
        self.build_bridges(bridges).await;
    }

    async fn get_piers(&self) -> Vec<[Pier; 4]> {
        self.cx
            .with_world(|world| get_piers(world, &self.parameters))
            .await
    }

    async fn get_bridges(&self, piers: Vec<[Pier; 4]>) -> Vec<Bridge> {
        piers
            .into_iter()
            .flat_map(|piers| {
                Bridge {
                    piers: piers.to_vec(),

                    bridge_type: BridgeType::Theoretical,
                }
                .validate()
            })
            .collect()
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

fn get_piers(world: &World, parameters: &RiverPierParameters) -> Vec<[Pier; 4]> {
    let mut out = vec![];
    for x in 0..world.width() {
        for y in 0..world.height() {
            for offset in [v2(1, 0), v2(0, 1), v2(-1, 0), v2(0, -1)].iter() {
                let from = v2(x, y);

                if let Some(to) = world.offset(&from, *offset) {
                    if let Some(pier) = is_pier(world, &from, &to, parameters) {
                        out.push(pier);
                    }
                }
            }
        }
    }
    out
}

fn is_pier(
    world: &World,
    from: &V2<usize>,
    to: &V2<usize>,
    parameters: &RiverPierParameters,
) -> Option<[Pier; 4]> {
    let from_cell = world.get_cell_unsafe(from);
    let to_cell = world.get_cell_unsafe(to);

    let from_elevation = from_cell.elevation;
    let to_elevation = to_cell.elevation;

    let sea_level = world.sea_level();

    if from_cell.river.here() {
        return None;
    }

    if from_elevation <= sea_level {
        return None;
    }

    if to_cell.river.longest_side() < parameters.min_navigable_river_width {
        return None;
    }

    if world.get_rise(from, to)?.abs() > parameters.max_gradient {
        return None;
    }

    if !has_launching_zone(world, from, &parameters.max_landing_zone_gradient) {
        return None;
    }

    let rotation = Rotation::from_positions(from, to).ok()?;
    Some([
        Pier {
            position: *from,
            elevation: from_elevation,
            platform: true,
            rotation,
            vehicle: Vehicle::None,
        },
        Pier {
            position: *to,
            elevation: to_elevation,
            platform: false,
            rotation,
            vehicle: Vehicle::None,
        },
        Pier {
            position: *to,
            elevation: to_elevation,
            platform: false,
            rotation,
            vehicle: Vehicle::Boat,
        },
        Pier {
            position: *to,
            elevation: to_elevation,
            platform: false,
            rotation,
            vehicle: Vehicle::Boat,
        },
    ])
}

fn has_launching_zone(
    world: &World,
    position: &V2<usize>,
    max_landing_zone_gradient: &f32,
) -> bool {
    world
        .get_adjacent_tiles_in_bounds(position)
        .iter()
        .any(|tile| {
            !world.is_sea(tile) && world.get_max_abs_rise(tile) <= *max_landing_zone_gradient
        })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use commons::async_trait::async_trait;
    use commons::edge::Edge;
    use commons::junction::PositionJunction;
    use commons::{v2, M};
    use futures::executor::block_on;

    use crate::bridges::Bridges;

    use super::*;

    struct Cx {
        bridges: Mutex<Bridges>,
        world: Mutex<World>,
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
            bridges: Mutex::default(),
            world: Mutex::new(world),
        }
    }

    fn parameters() -> RiverPierParameters {
        RiverPierParameters {
            min_navigable_river_width: 0.5,
            max_landing_zone_gradient: 1.5,
            max_gradient: 0.5,
        }
    }

    #[test]
    fn should_add_pier_into_river_right() {
        // Given
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 0.9, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(2, 1));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let river_piers = RiverPiers::new(cx, parameters());

        // When
        block_on(river_piers.new_game());

        // Then
        assert_eq!(
            *river_piers.cx.bridges.lock().unwrap(),
            hashmap! {
                Edge::new(v2(1, 1), v2(2, 1)) => hashset!{
                    Bridge{
                        piers: vec![
                            Pier{
                                position: v2(1, 1),
                                elevation: 1.0,
                                platform: true,
                                rotation: Rotation::Right,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(2, 1),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Right,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(2, 1),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Right,
                                vehicle: Vehicle::Boat,
                            },
                            Pier{
                                position: v2(2, 1),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Right,
                                vehicle: Vehicle::Boat,
                            },
                        ],
                        bridge_type: BridgeType::Theoretical
                    }
                }
            }
        );
    }

    #[test]
    fn should_add_pier_into_river_left() {
        // Given
        let mut world = World::new(
            M::from_vec(
                3,
                2,
                vec![
                    0.9, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(0, 0));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let river_piers = RiverPiers::new(cx, parameters());

        // When
        block_on(river_piers.new_game());

        // Then
        assert_eq!(
            *river_piers.cx.bridges.lock().unwrap(),
            hashmap! {
                Edge::new(v2(0, 0), v2(1, 0)) => hashset!{
                    Bridge{
                        piers: vec![
                            Pier{
                                position: v2(1, 0),
                                elevation: 1.0,
                                platform: true,
                                rotation: Rotation::Left,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(0, 0),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Left,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(0, 0),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Left,
                                vehicle: Vehicle::Boat,
                            },
                            Pier{
                                position: v2(0, 0),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Left,
                                vehicle: Vehicle::Boat,
                            },
                        ],
                        bridge_type: BridgeType::Theoretical
                    }
                }
            }
        );
    }

    #[test]
    fn should_add_pier_into_river_down() {
        // Given
        let mut world = World::new(
            M::from_vec(
                2,
                3,
                vec![
                    0.9, 1.0, //
                    1.0, 1.0, //
                    1.0, 1.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(0, 0));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let river_piers = RiverPiers::new(cx, parameters());

        // When
        block_on(river_piers.new_game());

        // Then
        assert_eq!(
            *river_piers.cx.bridges.lock().unwrap(),
            hashmap! {
                Edge::new(v2(0, 0), v2(0, 1)) => hashset!{
                    Bridge{
                        piers: vec![
                            Pier{
                                position: v2(0, 1),
                                elevation: 1.0,
                                platform: true,
                                rotation: Rotation::Down,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(0, 0),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Down,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(0, 0),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Down,
                                vehicle: Vehicle::Boat,
                            },
                            Pier{
                                position: v2(0, 0),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Down,
                                vehicle: Vehicle::Boat,
                            },
                        ],
                        bridge_type: BridgeType::Theoretical
                    }
                }
            }
        );
    }

    #[test]
    fn should_add_pier_into_river_up() {
        // Given
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 0.9, 1.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(1, 2));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let river_piers = RiverPiers::new(cx, parameters());

        // When
        block_on(river_piers.new_game());

        // Then
        assert_eq!(
            *river_piers.cx.bridges.lock().unwrap(),
            hashmap! {
                Edge::new(v2(1, 1), v2(1, 2)) => hashset!{
                    Bridge{
                        piers: vec![
                            Pier{
                                position: v2(1, 1),
                                elevation: 1.0,
                                platform: true,
                                rotation: Rotation::Up,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(1, 2),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Up,
                                vehicle: Vehicle::None,
                            },
                            Pier{
                                position: v2(1, 2),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Up,
                                vehicle: Vehicle::Boat,
                            },
                            Pier{
                                position: v2(1, 2),
                                elevation: 0.9,
                                platform: false,
                                rotation: Rotation::Up,
                                vehicle: Vehicle::Boat,
                            },
                        ],
                        bridge_type: BridgeType::Theoretical
                    }
                }
            }
        );
    }

    #[test]
    fn should_not_add_piers_into_stream() {
        // Given
        let mut world = World::new(M::from_element(3, 3, 1.0), 0.5);

        let mut river_1 = PositionJunction::new(v2(2, 1));
        river_1.junction.horizontal.width = 0.1;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let river_piers = RiverPiers::new(cx, parameters());

        // When
        block_on(river_piers.new_game());

        // Then
        assert_eq!(*river_piers.cx.bridges.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_not_add_pier_exceeding_max_gradient() {
        // Given
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 2.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(2, 1));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let river_piers = RiverPiers::new(cx, parameters());

        // When
        block_on(river_piers.new_game());

        // Then
        assert_eq!(*river_piers.cx.bridges.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_not_add_pier_with_no_landing_zone() {
        // Given
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    3.0, 1.0, 3.0, //
                    1.0, 1.0, 1.0, //
                    3.0, 1.0, 3.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(2, 1));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.horizontal.from = true;
        river_1.junction.horizontal.to = true;
        world.add_river(river_1);

        let cx = cx(world);

        let river_piers = RiverPiers::new(cx, parameters());

        // When
        block_on(river_piers.new_game());

        // Then
        assert_eq!(*river_piers.cx.bridges.lock().unwrap(), hashmap! {});
    }
}
