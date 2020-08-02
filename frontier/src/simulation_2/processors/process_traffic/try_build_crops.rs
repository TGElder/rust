use super::*;

use crate::game::traits::HasWorld;
use crate::resource::Resource;
use crate::world::WorldObject;
use commons::grid::Grid;

const FARM_RESOURCE: Resource = Resource::Crops;

pub fn try_build_crops(game: &dyn HasWorld, traffic: &TrafficSummary) -> Option<BuildInstruction> {
    let crop_routes = get_crop_routes(&traffic);
    if crop_routes.is_empty() {
        return None;
    }

    if !cell_is_free(game, &traffic.position) {
        return None;
    };

    let instruction = BuildInstruction {
        when: get_when(&crop_routes),
        what: Build::Crops {
            position: traffic.position,
            rotated: true,
        },
    };
    Some(instruction)
}

fn get_crop_routes(traffic: &TrafficSummary) -> Vec<&RouteSummary> {
    traffic
        .routes
        .iter()
        .filter(|route| route.resource == FARM_RESOURCE && route.destination == traffic.position)
        .collect()
}

fn cell_is_free(world: &dyn HasWorld, position: &V2<usize>) -> bool {
    let cell = unwrap_or!(world.world().get_cell(&position), return false);
    cell.object == WorldObject::None
}

fn get_when(routes: &[&RouteSummary]) -> u128 {
    get_first_visit_route(routes).first_visit
}

fn get_first_visit_route<'a>(routes: &[&'a RouteSummary]) -> &'a RouteSummary {
    routes.iter().min_by_key(|route| route.first_visit).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::world::World;
    use commons::grid::Grid;
    use commons::{v2, M};
    use std::time::Duration;

    fn world() -> World {
        World::new(M::zeros(3, 3), 0.0)
    }

    #[test]
    fn should_build_crops_if_route_for_crops_end_here() {
        // Given
        let traffic = TrafficSummary {
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
            }],
            adjacent: vec![],
        };

        // When
        let result = try_build_crops(&world(), &traffic);

        // Then
        let instruction = result.expect("No build instruction!");
        assert_eq!(
            instruction.what,
            Build::Crops {
                position: v2(1, 2),
                rotated: true,
            }
        );
    }

    #[test]
    fn when_should_be_first_crops_route_to_reach_cell() {
        // Given
        let traffic = TrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![
                RouteSummary {
                    traffic: 1,
                    origin: v2(0, 1),
                    destination: v2(1, 2),
                    nation: String::default(),
                    first_visit: 200,
                    duration: Duration::default(),
                    resource: Resource::Crops,
                },
                RouteSummary {
                    traffic: 1,
                    origin: v2(0, 1),
                    destination: v2(1, 2),
                    nation: String::default(),
                    first_visit: 100,
                    duration: Duration::default(),
                    resource: Resource::Crops,
                },
                RouteSummary {
                    traffic: 1,
                    origin: v2(0, 1),
                    destination: v2(1, 2),
                    nation: String::default(),
                    first_visit: 0,
                    duration: Duration::default(),
                    resource: Resource::Pasture,
                },
            ],
            adjacent: vec![],
        };

        // When
        let result = try_build_crops(&world(), &traffic);

        // Then
        let instruction = result.expect("No build instruction!");
        assert_eq!(instruction.when, 100);
    }

    #[test]
    fn should_not_build_crops_if_route_for_crops_does_not_end_here() {
        // Given
        let traffic = TrafficSummary {
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
            }],
            adjacent: vec![],
        };

        // When
        let result = try_build_crops(&world(), &traffic);

        // Then
        assert_eq!(result, None);
    }

    #[test]
    fn should_not_build_crops_if_route_not_for_crops_ends_here() {
        // Given
        let traffic = TrafficSummary {
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
            }],
            adjacent: vec![],
        };

        // When
        let result = try_build_crops(&world(), &traffic);

        // Then
        assert_eq!(result, None);
    }

    #[test]
    fn should_not_build_crops_if_cell_already_occupied() {
        // Given
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).object = WorldObject::Crop { rotated: true };

        let traffic = TrafficSummary {
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
            }],
            adjacent: vec![],
        };

        // When
        let result = try_build_crops(&world, &traffic);

        // Then
        assert_eq!(result, None);
    }
}
