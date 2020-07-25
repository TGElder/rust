use super::*;

use crate::game::traits::Nations;
use crate::settlement::{Settlement, SettlementClass::Town};
use commons::v2;
use std::convert::TryInto;
use std::time::Duration;

const TRAFFIC_TO_POPULATION: f64 = 0.5;

pub fn try_build_destination_town<G>(
    game: &mut G,
    traffic: &TrafficSummary,
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
            settlement: get_settlement(game, &traffic.routes),
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
}

fn get_candidate_positions(tiles: &[Tile]) -> Vec<V2<usize>> {
    tiles
        .iter()
        .filter(|tile| !tile.sea)
        .filter(|tile| tile.visible)
        .map(|tile| tile.position)
        .collect()
}

fn get_settlement<G>(game: &mut G, routes: &[RouteSummary]) -> Settlement
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
        current_population: 0.0,
        target_population: get_target_population(routes),
        gap_half_life: get_gap_half_life(routes),
        last_population_update_micros: get_when(routes),
    }
}

fn get_first_visit_route(routes: &[RouteSummary]) -> &RouteSummary {
    routes.iter().min_by_key(|route| route.first_visit).unwrap()
}

fn get_traffic(routes: &[RouteSummary]) -> usize {
    routes.iter().map(|route| route.traffic).sum()
}

fn get_target_population(routes: &[RouteSummary]) -> f64 {
    get_traffic(routes) as f64 * TRAFFIC_TO_POPULATION
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
        let traffic = TrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
            }],
            adjacent: vec![
                Tile {
                    position: v2(0, 2),
                    settlement: None,
                    sea: false,
                    visible: true,
                },
                Tile {
                    position: v2(1, 1),
                    settlement: None,
                    sea: false,
                    visible: true,
                },
            ],
        };

        let instruction = try_build_destination_town(&mut game, &traffic);

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
            assert!(settlement.current_population.almost(&0.0));
            // Settlement target population is traffic * TRAFFIC_TO_POPULATION
            assert!(settlement
                .target_population
                .almost(&(3.0 * TRAFFIC_TO_POPULATION)));
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
        let traffic = TrafficSummary {
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
                },
                RouteSummary {
                    traffic: 3,
                    origin: v2(0, 2),
                    destination: v2(1, 2),
                    nation: "Wales".to_string(),
                    first_visit: 101,
                    duration: Duration::from_micros(101),
                },
            ],
            adjacent: vec![
                Tile {
                    position: v2(0, 2),
                    settlement: None,
                    sea: false,
                    visible: true,
                },
                Tile {
                    position: v2(1, 1),
                    settlement: None,
                    sea: false,
                    visible: true,
                },
            ],
        };

        let instruction = try_build_destination_town(&mut game, &traffic);

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
            // Settlement target population is traffic for all routes * TRAFFIC_TO_POPULATION
            assert!(settlement
                .target_population
                .almost(&(9.0 * TRAFFIC_TO_POPULATION)));
            // Gap half life is average round-trip duration of routes to position
            assert_eq!(settlement.gap_half_life, Duration::from_micros(303));
            // Last population update is same as when (build time)
            assert_eq!(settlement.last_population_update_micros, 101);
        } else {
            panic!("No settlement build instruction!");
        }
    }

    fn should_not_build_town(traffic: TrafficSummary) {
        // Given
        let mut game = nations();

        // When
        let instruction = try_build_destination_town(&mut game, &traffic);

        // Then
        assert_eq!(instruction, None);
    }

    #[test]
    fn should_not_build_town_if_no_route_ends_at_position() {
        should_not_build_town(TrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(2, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                settlement: None,
                sea: false,
                visible: true,
            }],
        });
    }

    #[test]
    fn should_not_build_town_if_position_already_controlled() {
        should_not_build_town(TrafficSummary {
            position: v2(1, 2),
            controller: Some(v2(1, 1)),
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                settlement: None,
                sea: false,
                visible: true,
            }],
        });
    }

    #[test]
    fn should_not_build_town_in_sea() {
        should_not_build_town(TrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                settlement: None,
                sea: true,
                visible: true,
            }],
        });
    }

    #[test]
    fn should_not_build_town_on_invisible_tile() {
        should_not_build_town(TrafficSummary {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
                duration: Duration::from_micros(101),
            }],
            adjacent: vec![Tile {
                position: v2(0, 2),
                settlement: None,
                sea: false,
                visible: false,
            }],
        });
    }
}
