use super::*;

use crate::build_service::{Build, BuildInstruction, BuildQueue};

const HANDLE: &str = "build_road";
const ROAD_THRESHOLD: usize = 8;

pub struct BuildRoad<B>
where
    B: BuildQueue,
{
    builder: UpdateSender<B>,
}

impl<B> Processor for BuildRoad<B>
where
    B: BuildQueue,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let (edge, road_status, routes) = match instruction {
            Instruction::EdgeTraffic {
                edge,
                road_status,
                routes,
            } => (edge, road_status, routes),
            _ => return state,
        };
        if *road_status != RoadStatus::Suitable {
            return state;
        }
        if get_traffic(routes) < ROAD_THRESHOLD {
            return state;
        }
        let instruction = BuildInstruction {
            when: get_when(routes.clone()),
            what: Build::Road(*edge),
        };
        self.build(instruction);
        state
    }
}

impl<B> BuildRoad<B>
where
    B: BuildQueue,
{
    pub fn new(builder: &UpdateSender<B>) -> BuildRoad<B> {
        BuildRoad {
            builder: builder.clone_with_handle(HANDLE),
        }
    }

    fn build(&mut self, instruction: BuildInstruction) {
        block_on(async {
            self.builder
                .update(|builder| builder.queue(instruction))
                .await
        });
    }
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

    use commons::edge::Edge;
    use commons::update::UpdateProcess;
    use commons::v2;

    #[test]
    fn should_build_road_if_traffic_exceeds_threshold() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let build_queue = UpdateProcess::new(vec![]);
        let mut processor = BuildRoad::new(&build_queue.tx());

        // When
        processor.process(
            State::default(),
            &Instruction::EdgeTraffic {
                edge,
                road_status: RoadStatus::Suitable,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD,
                    first_visit: 0,
                }],
            },
        );

        // Then
        let build_queue = build_queue.shutdown();
        if let Some(BuildInstruction {
            what: Build::Road(actual),
            ..
        }) = build_queue.get(0)
        {
            assert_eq!(*actual, edge);
        } else {
            panic!("No build instruction!");
        }
    }

    #[test]
    fn should_not_build_road_if_traffic_doesnt_exceed_threshold() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let build_queue = UpdateProcess::new(vec![]);
        let mut processor = BuildRoad::new(&build_queue.tx());

        // When
        processor.process(
            State::default(),
            &Instruction::EdgeTraffic {
                edge,
                road_status: RoadStatus::Suitable,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD - 1,
                    first_visit: 0,
                }],
            },
        );

        // Then
        let build_queue = build_queue.shutdown();
        assert_eq!(build_queue, vec![]);
    }

    #[test]
    fn should_not_build_road_if_already_built() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let build_queue = UpdateProcess::new(vec![]);
        let mut processor = BuildRoad::new(&build_queue.tx());

        // When
        processor.process(
            State::default(),
            &Instruction::EdgeTraffic {
                edge,
                road_status: RoadStatus::Built,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD,
                    first_visit: 0,
                }],
            },
        );

        // Then
        let build_queue = build_queue.shutdown();
        assert_eq!(build_queue, vec![]);
    }

    #[test]
    fn should_not_build_road_if_unsuitable_edge() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let build_queue = UpdateProcess::new(vec![]);
        let mut processor = BuildRoad::new(&build_queue.tx());

        // When
        processor.process(
            State::default(),
            &Instruction::EdgeTraffic {
                edge,
                road_status: RoadStatus::Unsuitable,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD,
                    first_visit: 0,
                }],
            },
        );

        // Then
        let build_queue = build_queue.shutdown();
        assert_eq!(build_queue, vec![]);
    }

    #[test]
    fn when_should_be_time_at_which_traffic_exceeded() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let build_queue = UpdateProcess::new(vec![]);
        let mut processor = BuildRoad::new(&build_queue.tx());

        let routes = (1..=ROAD_THRESHOLD)
            .map(|i| EdgeRouteSummary {
                traffic: 1,
                first_visit: (i * 10) as u128,
            })
            .collect();

        // When
        processor.process(
            State::default(),
            &Instruction::EdgeTraffic {
                edge,
                road_status: RoadStatus::Suitable,
                routes,
            },
        );

        // Then
        let build_queue = build_queue.shutdown();
        let expected = ROAD_THRESHOLD as u128 * 10;
        if let Some(BuildInstruction { when: actual, .. }) = build_queue.get(0) {
            assert_eq!(*actual, expected);
        } else {
            panic!("No build instruction!");
        }
    }
}
