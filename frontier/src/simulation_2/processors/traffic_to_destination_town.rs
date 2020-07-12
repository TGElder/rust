use super::*;

use crate::build_service::{Build, BuildInstruction, BuildQueue};
use crate::game::traits::Nations;
use crate::settlement::{Settlement, SettlementClass::Town};
use commons::v2;

const HANDLE: &str = "traffic_to_destination_town";
const TRAFFIC_TO_POPULATION: f64 = 0.5;

pub struct TrafficToDestinationTown<G, B>
where
    G: Nations,
    B: BuildQueue,
{
    game: UpdateSender<G>,
    builder: UpdateSender<B>,
}

impl<G, B> Processor for TrafficToDestinationTown<G, B>
where
    G: Nations,
    B: BuildQueue,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        self.try_build(instruction);
        state
    }
}

impl<G, B> TrafficToDestinationTown<G, B>
where
    G: Nations,
    B: BuildQueue,
{
    pub fn new(
        game: &UpdateSender<G>,
        builder: &UpdateSender<B>,
    ) -> TrafficToDestinationTown<G, B> {
        TrafficToDestinationTown {
            game: game.clone_with_handle(HANDLE),
            builder: builder.clone_with_handle(HANDLE),
        }
    }

    fn try_build(&mut self, instruction: &Instruction) {
        let (position, controller, routes, adjacent) = match instruction {
            Instruction::Traffic {
                position,
                controller,
                routes,
                adjacent,
            } => (position, controller, routes, adjacent),
            _ => return,
        };
        if !should_build(&position, &controller, &routes) {
            return;
        }

        let candidate_positions = get_candidate_positions(&adjacent);
        if candidate_positions.is_empty() {
            return;
        }

        let instruction = BuildInstruction {
            when: get_when(routes),
            what: Build::Settlement {
                candidate_positions,
                settlement: self.get_settlement(routes.clone()),
            },
        };
        self.build(instruction);
    }

    fn get_settlement(&mut self, routes: Vec<RouteSummary>) -> Settlement {
        block_on(async {
            self.game
                .update(move |game| get_settlement(game, routes))
                .await
        })
    }

    fn build(&mut self, instruction: BuildInstruction) {
        block_on(async {
            self.builder
                .update(|builder| builder.queue(instruction))
                .await
        });
    }
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

fn get_settlement<G>(game: &mut G, routes: Vec<RouteSummary>) -> Settlement
where
    G: Nations,
{
    let first_visit_route = get_first_visit_route(&routes);
    let nation = game.mut_nation_unsafe(&first_visit_route.nation);

    Settlement {
        class: Town,
        position: v2(0, 0),
        name: nation.get_town_name(),
        nation: first_visit_route.nation.clone(),
        current_population: 0.0,
        target_population: get_target_population(&routes),
        gap_half_life: None,
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

fn get_when(routes: &[RouteSummary]) -> u128 {
    get_first_visit_route(routes).first_visit
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::nation::{Nation, NationDescription};
    use commons::almost::Almost;
    use commons::same_elements;
    use commons::update::UpdateProcess;
    use isometric::Color;
    use std::collections::HashMap;

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

    impl BuildQueue for Vec<BuildInstruction> {
        fn queue(&mut self, build_instruction: BuildInstruction) {
            self.push(build_instruction)
        }
    }

    #[test]
    fn should_build_town_if_single_route_ends_at_position() {
        // Given
        let game = UpdateProcess::new(nations());
        let build_queue = UpdateProcess::new(vec![]);
        let mut processor = TrafficToDestinationTown::new(&game.tx(), &build_queue.tx());

        // When
        let instruction = Instruction::Traffic {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
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
        processor.process(State::default(), &instruction);
        let build_queue = build_queue.shutdown();

        // Then
        if let Some(BuildInstruction {
            when,
            what:
                Build::Settlement {
                    candidate_positions,
                    settlement,
                },
        }) = build_queue.get(0)
        {
            // When is first visit
            assert_eq!(*when, 101);
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
        } else {
            panic!("No settlement build instruction!");
        }

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_build_town_if_multiple_routes_end_at_position() {
        // Given
        let game = UpdateProcess::new(nations());
        let build_queue = UpdateProcess::new(vec![]);
        let mut processor = TrafficToDestinationTown::new(&game.tx(), &build_queue.tx());

        // When
        let instruction = Instruction::Traffic {
            position: v2(1, 2),
            controller: None,
            routes: vec![
                RouteSummary {
                    traffic: 6,
                    origin: v2(0, 0),
                    destination: v2(1, 2),
                    nation: "Scotland".to_string(),
                    first_visit: 202,
                },
                RouteSummary {
                    traffic: 3,
                    origin: v2(0, 2),
                    destination: v2(1, 2),
                    nation: "Wales".to_string(),
                    first_visit: 101,
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
        processor.process(State::default(), &instruction);
        let build_queue = build_queue.shutdown();

        // Then
        if let Some(BuildInstruction {
            when,
            what: Build::Settlement { settlement, .. },
        }) = build_queue.get(0)
        {
            // When is first visit in any route
            assert_eq!(*when, 101);
            // Settlement nation is nation with lowest first visit
            assert_eq!(settlement.nation, "Wales".to_string());
            assert_eq!(settlement.name, "Swansea".to_string());
            // Settlement target population is traffic for all routes * TRAFFIC_TO_POPULATION
            assert!(settlement
                .target_population
                .almost(&(9.0 * TRAFFIC_TO_POPULATION)));
        } else {
            panic!("No settlement build instruction!");
        }

        // Finally
        game.shutdown();
    }

    fn should_not_build_town(instruction: Instruction) {
        // Given
        let game = UpdateProcess::new(nations());
        let build_queue = UpdateProcess::new(vec![]);
        let mut processor = TrafficToDestinationTown::new(&game.tx(), &build_queue.tx());

        // When
        processor.process(State::default(), &instruction);
        let build_queue = build_queue.shutdown();

        // Then
        assert_eq!(build_queue, vec![]);

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_not_build_town_if_no_route_ends_at_position() {
        should_not_build_town(Instruction::Traffic {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(2, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
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
        should_not_build_town(Instruction::Traffic {
            position: v2(1, 2),
            controller: Some(v2(1, 1)),
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
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
        should_not_build_town(Instruction::Traffic {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
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
        should_not_build_town(Instruction::Traffic {
            position: v2(1, 2),
            controller: None,
            routes: vec![RouteSummary {
                traffic: 3,
                origin: v2(0, 0),
                destination: v2(1, 2),
                nation: "Scotland".to_string(),
                first_visit: 101,
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
