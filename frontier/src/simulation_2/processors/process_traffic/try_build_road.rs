use super::*;

use crate::game::traits::HasWorld;

const ROAD_THRESHOLD: usize = 8;

pub fn try_build_road(
    world: &mut dyn HasWorld,
    traffic: &EdgeTrafficSummary,
) -> Option<BuildInstruction> {
    if get_traffic(&traffic.routes) < ROAD_THRESHOLD {
        return None;
    }
    let when = get_when(traffic.routes.clone());
    match traffic.road_status {
        RoadStatus::Suitable => (),
        RoadStatus::Planned(at) if at > when => (),
        _ => return None,
    }
    world.world_mut().plan_road(&traffic.edge, true, when);
    let instruction = BuildInstruction {
        when,
        what: Build::Road(traffic.edge),
    };
    Some(instruction)
}

fn get_traffic(routes: &[EdgeRouteSummary]) -> usize {
    routes.iter().map(|route| route.traffic).sum()
}

fn get_when(mut routes: Vec<EdgeRouteSummary>) -> u128 {
    routes.sort_by_key(|route| route.first_visit);
    let mut traffic_cum = 0;
    for route in routes {
        traffic_cum += route.traffic;
        if traffic_cum >= ROAD_THRESHOLD {
            return route.first_visit;
        }
    }
    panic!(
        "Total traffic {} does not exceed threshold for building road {}",
        traffic_cum, ROAD_THRESHOLD
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::world::World;
    use commons::edge::Edge;
    use commons::{v2, M};

    fn world() -> World {
        World::new(M::zeros(4, 4), 0.5)
    }

    #[test]
    fn should_build_road_if_traffic_exceeds_threshold() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut world = world();

        // When
        let instruction = try_build_road(
            &mut world,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Suitable,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert_eq!(world.road_planned(&edge), Some(0));
        if let Some(BuildInstruction {
            what: Build::Road(actual),
            when: 0,
        }) = instruction
        {
            assert_eq!(actual, edge);
        } else {
            panic!("No build instruction!");
        }
    }

    #[test]
    fn should_not_build_road_if_traffic_doesnt_exceed_threshold() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut world = world();

        // When
        let instruction = try_build_road(
            &mut world,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Suitable,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD - 1,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert!(world.road_planned(&edge).is_none());
        assert_eq!(instruction, None);
    }

    #[test]
    fn should_not_build_road_if_already_built() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut world = world();

        // When
        let instruction = try_build_road(
            &mut world,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Built,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert!(world.road_planned(&edge).is_none());
        assert_eq!(instruction, None);
    }

    #[test]
    fn should_not_build_road_planned_earlier() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut world = world();

        // When
        let instruction = try_build_road(
            &mut world,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Planned(0),
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD,
                    first_visit: 100,
                }],
            },
        );

        // Then
        assert!(world.road_planned(&edge).is_none());
        assert_eq!(instruction, None);
    }

    #[test]
    fn should_build_road_planned_later() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut world = world();

        // When
        let instruction = try_build_road(
            &mut world,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Planned(100),
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert_eq!(world.road_planned(&edge), Some(0));
        if let Some(BuildInstruction {
            what: Build::Road(actual),
            when: 0,
        }) = instruction
        {
            assert_eq!(actual, edge);
        } else {
            panic!("No build instruction!");
        }
    }

    #[test]
    fn should_not_build_road_if_unsuitable_edge() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut world = world();

        // When
        let instruction = try_build_road(
            &mut world,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Unsuitable,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert!(world.road_planned(&edge).is_none());
        assert_eq!(instruction, None);
    }

    #[test]
    fn when_should_be_time_at_which_traffic_exceeded() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut world = world();

        let routes = (1..=ROAD_THRESHOLD)
            .map(|i| EdgeRouteSummary {
                traffic: 1,
                first_visit: (i * 10) as u128,
            })
            .collect();

        // When
        let instruction = try_build_road(
            &mut world,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Suitable,
                routes,
            },
        );

        // Then
        let expected = ROAD_THRESHOLD as u128 * 10;
        assert_eq!(world.road_planned(&edge), Some(expected));
        if let Some(BuildInstruction { when: actual, .. }) = instruction {
            assert_eq!(actual, expected);
        } else {
            panic!("No build instruction!");
        }
    }
}
