use super::*;

use crate::game::traits::Nations;
use crate::settlement::{Settlement, SettlementClass::Town};
use commons::v2;
use std::convert::TryInto;
use std::time::Duration;

pub fn try_build_town<G>(
    game: &mut G,
    traffic: &PositionTrafficSummary,
    initial_population: &f64,
) -> Option<BuildInstruction>
where
    G: Nations,
{
    if !should_build(&traffic.position, &traffic.controller, &traffic.routes) {
        return None;
    }

    let candidate_positions = get_candidate_positions(&traffic.adjacent);
    if candidate_positions.is_empty() {
        return None;
    }

    let instruction = BuildInstruction {
        when: get_when(&traffic.routes),
        what: Build::Settlement {
            candidate_positions,
            settlement: get_settlement(game, &traffic.routes, *initial_population),
        },
    };
    Some(instruction)
}

fn should_build(
    position: &V2<usize>,
    controller: &Option<V2<usize>>,
    routes: &[RouteSummary],
) -> bool {
    if controller.is_some() {
        return false;
    }
    routes.iter().any(|route| route.destination == *position)
        || routes.iter().any(|route| route.ports.contains(position))
}

fn get_candidate_positions(tiles: &[Tile]) -> Vec<V2<usize>> {
    tiles
        .iter()
        .filter(|tile| !tile.sea)
        .filter(|tile| tile.visible)
        .map(|tile| tile.position)
        .collect()
}

fn get_settlement<G>(game: &mut G, routes: &[RouteSummary], initial_population: f64) -> Settlement
where
    G: Nations,
{
    let first_visit_route = get_first_visit_route(routes);
    let nation = game.mut_nation_unsafe(&first_visit_route.nation);

    Settlement {
        class: Town,
        position: v2(0, 0),
        name: nation.get_town_name(),
        nation: first_visit_route.nation.clone(),
        current_population: initial_population,
        target_population: 0.0,
        gap_half_life: get_gap_half_life(routes),
        last_population_update_micros: get_when(routes),
    }
}

fn get_first_visit_route(routes: &[RouteSummary]) -> &RouteSummary {
    routes.iter().min_by_key(|route| route.first_visit).unwrap()
}

fn get_gap_half_life(routes: &[RouteSummary]) -> Duration {
    let total: Duration = routes.iter().map(|route| route.duration).sum();
    let count: u32 = routes.iter().count().try_into().unwrap();
    (total / count) * 2
}

fn get_when(routes: &[RouteSummary]) -> u128 {
    get_first_visit_route(routes).first_visit
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::nation::{Nation, NationDescription};
    use crate::resource::Resource;
    use commons::almost::Almost;
    use commons::same_elements;
    use isometric::Color;
    use std::collections::HashMap;
    use std::time::Duration;

    fn scotland() -> Nation {
        Nation::from_description(&NationDescription {
            name: "Scotland".to_string(),
            color: Color::transparent(),
            skin_color: Color::transparent(),
            town_name_file: "test/resources/names/scotland".to_string(),
        })
    }

    fn wales() -> Nation {
        Nation::from_description(&NationDescription {
            name: "Wales".to_string(),
            color: Color::transparent(),
            skin_color: Color::transparent(),
            town_name_file: "test/resources/names/wales".to_string(),
        })
    }

    fn nations() -> HashMap<String, Nation> {
        hashmap! {
            "Scotland".to_string() => scotland(),
            "Wales".to_string() => wales()
        }
    }

    #[test]
    fn should_build_town_if_single_route_ends_at_position() {
        // Given
        let mut game = nations();

        // When
        let traffic = PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
                resource: Resource::Pasture,
                ports: hashset! {},
            }],
            adjacent: vec![
                Tile {
                    position: v2(0, 2),
                    sea: false,
                    visible: true,
                },
                Tile {
                    position: v2(1, 1),
                    sea: false,
                    visible: true,
                },
            ],
        };

        let instruction = try_build_town(&mut game, &traffic, &0.5);

        // Then
        if let Some(BuildInstruction {
            when,
            what:
                Build::Settlement {
                    candidate_positions,
                    settlement,
                },
        }) = instruction
        {
            // When is first visit
            assert_eq!(when, 101);
            // Each adjacent tile is candidate position
            assert!(same_elements(&candidate_positions, &[v2(0, 2), v2(1, 1)]));
            assert_eq!(settlement.class, Town);
            assert_eq!(settlement.nation, "Scotland".to_string());
            assert_eq!(settlement.name, "Edinburgh".to_string());
            assert!(settlement.current_population.almost(&0.5));
            assert!(settlement.target_population.almost(&0.0));
            // Gap half life is average round-trip duration of routes to position
            assert_eq!(settlement.gap_half_life, Duration::from_micros(202));
            // Last population update is same as when (build time)
            assert_eq!(settlement.last_population_update_micros, 101);
        } else {
            panic!("No settlement build instruction!");
        }
    }

    #[test]
    fn should_build_town_if_multiple_routes_end_at_position() {
        // Given
        let mut game = nations();

        // When
        let traffic = PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![
                RouteSummary {
                    traffic: 6,
                    origin: v2(0, 0),
                    destination: v2(1, 2),
                    nation: "Scotland".to_string(),
                    first_visit: 202,
                    duration: Duration::from_micros(202),
                    resource: Resource::Pasture,
                    ports: hashset! {},
                },
                RouteSummary {
                    traffic: 3,
                    origin: v2(0, 2),
                    destination: v2(1, 2),
                    nation: "Wales".to_string(),
                    first_visit: 101,
                    duration: Duration::from_micros(101),
                    resource: Resource::Pasture,
                    ports: hashset! {},
                },
            ],
            adjacent: vec![
                Tile {
                    position: v2(0, 2),
                    sea: false,
                    visible: true,
                },
                Tile {
                    position: v2(1, 1),
                    sea: false,
                    visible: true,
                },
            ],
        };

        let instruction = try_build_town(&mut game, &traffic, &0.5);

        // Then
        if let Some(BuildInstruction {
            when,
            what: Build::Settlement { settlement, .. },
        }) = instruction
        {
            // When is first visit in any route
            assert_eq!(when, 101);
            // Settlement nation is nation with lowest first visit
            assert_eq!(settlement.nation, "Wales".to_string());
            assert_eq!(settlement.name, "Swansea".to_string());
            // Gap half life is average round-trip duration of routes to position
            assert_eq!(settlement.gap_half_life, Duration::from_micros(303));
            // Last population update is same as when (build time)
            assert_eq!(settlement.last_population_update_micros, 101);
        } else {
            panic!("No settlement build instruction!");
        }
    }

    #[test]
    fn should_build_town_if_any_route_uses_position_as_port() {
        // Given
        let mut game = nations();

        // When
        let traffic = PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(2, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
                resource: Resource::Pasture,
                ports: hashset! {v2(1, 2)},
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                sea: false,
                visible: true,
            }],
        };

        let instruction = try_build_town(&mut game, &traffic, &0.5);

        match instruction {
            Some(BuildInstruction {
                what: Build::Settlement { .. },
                ..
            }) => (),
            _ => panic!("No settlement build instruction!"),
        }
    }

    fn should_not_build_town(traffic: PositionTrafficSummary) {
        // Given
        let mut game = nations();

        // When
        let instruction = try_build_town(&mut game, &traffic, &0.5);

        // Then
        assert_eq!(instruction, None);
    }

    #[test]
    fn should_not_build_town_if_no_route_ends_at_position() {
        should_not_build_town(PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(2, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
                resource: Resource::Pasture,
                ports: hashset! {},
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                sea: false,
                visible: true,
            }],
        });
    }

    #[test]
    fn should_not_build_town_if_position_already_controlled() {
        should_not_build_town(PositionTrafficSummary {
            position: v2(1, 2),
            controller: Some(v2(1, 1)),
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
                resource: Resource::Pasture,
                ports: hashset! {},
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                sea: false,
                visible: true,
            }],
        });
    }

    #[test]
    fn should_not_build_town_in_sea() {
        should_not_build_town(PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
                resource: Resource::Pasture,
                ports: hashset! {},
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                sea: true,
                visible: true,
            }],
        });
    }

    #[test]
    fn should_not_build_town_on_invisible_tile() {
        should_not_build_town(PositionTrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
                resource: Resource::Pasture,
                ports: hashset! {},
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                sea: false,
                visible: false,
            }],
        });
    }
}
