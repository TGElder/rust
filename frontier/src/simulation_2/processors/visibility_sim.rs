use super::*;
use crate::game::traits::VisiblePositions;
use crate::game::{CaptureEvent, CellSelection, GameEvent, GameEventConsumer, GameState};
use commons::grid::Grid;
use isometric::Event;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};

pub struct VisibilityMessage {
    position: V2<usize>,
}

pub struct VisibilitySim<G>
where
    G: VisiblePositions,
{
    tx: Sender<VisibilityMessage>,
    rx: Receiver<VisibilityMessage>,
    game: UpdateSender<G>,
}

impl<G> Processor for VisibilitySim<G>
where
    G: VisiblePositions,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Step => (),
            _ => return state,
        };
        let messages = self.get_messages();
        if !messages.is_empty() {
            state.instructions.push(Instruction::TotalVisiblePositions(
                self.total_visible_positions(),
            ));
        }
        for position in get_traffic_positions(&state, &messages) {
            state.instructions.push(Instruction::GetTraffic(position));
        }
        state
    }
}

impl<G> VisibilitySim<G>
where
    G: VisiblePositions,
{
    pub fn new(game: &UpdateSender<G>) -> VisibilitySim<G> {
        let (tx, rx) = channel();
        VisibilitySim {
            tx,
            rx,
            game: game.clone_with_handle(HANDLE),
        }
    }

    pub fn consumer(&self) -> VisibilitySimConsumer {
        VisibilitySimConsumer::new(&self.tx)
    }

    fn get_messages(&mut self) -> Vec<VisibilityMessage> {
        let mut out = vec![];
        while let Ok(message) = self.rx.try_recv() {
            out.push(message);
        }
        out
    }

    fn total_visible_positions(&mut self) -> usize {
        block_on(async { self.game.update(|game| total_visible_positions(game)).await })
    }
}

fn total_visible_positions<G>(game: &mut G) -> usize
where
    G: VisiblePositions,
{
    game.total_visible_positions()
}

fn get_traffic_positions(state: &State, messages: &[VisibilityMessage]) -> HashSet<V2<usize>> {
    messages
        .iter()
        .map(|VisibilityMessage { position, .. }| position)
        .flat_map(|position| state.traffic.expand_position(position))
        .filter(|position| should_get_traffic(state, position))
        .collect()
}

fn should_get_traffic(state: &State, position: &V2<usize>) -> bool {
    !state.traffic.get_cell_unsafe(position).is_empty()
}

const HANDLE: &str = "visibility_sim_consumer";

pub struct VisibilitySimConsumer {
    tx: Sender<VisibilityMessage>,
}

impl VisibilitySimConsumer {
    pub fn new(tx: &Sender<VisibilityMessage>) -> VisibilitySimConsumer {
        VisibilitySimConsumer { tx: tx.clone() }
    }

    fn send_messages(&mut self, cells: &[V2<usize>]) {
        for cell in cells {
            self.send_message(*cell);
        }
    }

    fn send_message(&mut self, cell: V2<usize>) {
        self.tx.send(VisibilityMessage { position: cell }).unwrap();
    }
}

impl GameEventConsumer for VisibilitySimConsumer {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::CellsRevealed {
            selection: CellSelection::Some(cells),
            ..
        } = event
        {
            self.send_messages(cells)
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::route::RouteKey;
    use crate::world::Resource;
    use commons::grid::Grid;
    use commons::same_elements;
    use commons::update::UpdateProcess;
    use commons::v2;
    use std::collections::HashSet;

    #[test]
    fn should_append_get_traffic_instruction_for_positions_surrounding_revealed_cell_with_traffic()
    {
        // Given
        let game = UpdateProcess::new(0);
        let mut processor = VisibilitySim::new(&game.tx());
        let mut consumer = processor.consumer();

        let mut traffic = Traffic::new(4, 4, HashSet::with_capacity(0));
        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Stone,
            destination: v2(2, 2),
        };
        traffic.mut_cell_unsafe(&v2(0, 1)).insert(route_key);
        traffic.mut_cell_unsafe(&v2(1, 1)).insert(route_key);
        traffic.mut_cell_unsafe(&v2(2, 1)).insert(route_key);
        traffic.mut_cell_unsafe(&v2(0, 2)).insert(route_key);
        traffic.mut_cell_unsafe(&v2(1, 2)).insert(route_key);
        traffic.mut_cell_unsafe(&v2(2, 2)).insert(route_key);
        traffic.mut_cell_unsafe(&v2(0, 3)).insert(route_key);
        traffic.mut_cell_unsafe(&v2(1, 3)).insert(route_key);
        traffic.mut_cell_unsafe(&v2(2, 3)).insert(route_key);
        let state = State {
            traffic,
            ..State::default()
        };

        // When
        consumer.consume_game_event(
            &GameState::default(),
            &GameEvent::CellsRevealed {
                selection: CellSelection::Some(vec![v2(1, 2)]),
                by: "",
            },
        );
        let state = processor.process(state, &Instruction::Step);

        // Then
        assert!(same_elements(
            &state.instructions,
            &[
                Instruction::TotalVisiblePositions(0),
                Instruction::GetTraffic(v2(0, 1)),
                Instruction::GetTraffic(v2(1, 1)),
                Instruction::GetTraffic(v2(2, 1)),
                Instruction::GetTraffic(v2(0, 2)),
                Instruction::GetTraffic(v2(1, 2)),
                Instruction::GetTraffic(v2(2, 2)),
                Instruction::GetTraffic(v2(0, 3)),
                Instruction::GetTraffic(v2(1, 3)),
                Instruction::GetTraffic(v2(2, 3)),
            ]
        ));

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_not_append_get_traffic_instruction_for_positions_surrounding_revealed_cell_without_traffic(
    ) {
        // Given
        let game = UpdateProcess::new(0);
        let mut processor = VisibilitySim::new(&game.tx());
        let mut consumer = processor.consumer();

        let state = State {
            traffic: Traffic::new(4, 4, HashSet::with_capacity(0)),
            ..State::default()
        };

        // When
        consumer.consume_game_event(
            &GameState::default(),
            &GameEvent::CellsRevealed {
                selection: CellSelection::Some(vec![v2(1, 2)]),
                by: "",
            },
        );
        let state = processor.process(state, &Instruction::Step);

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::TotalVisiblePositions(0)]
        );

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_not_append_duplicate_instructions() {
        // Given
        let game = UpdateProcess::new(0);
        let mut processor = VisibilitySim::new(&game.tx());
        let mut consumer = processor.consumer();

        let mut traffic = Traffic::new(4, 4, HashSet::with_capacity(0));
        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Stone,
            destination: v2(2, 2),
        };
        traffic.mut_cell_unsafe(&v2(2, 2)).insert(route_key);
        let state = State {
            traffic,
            ..State::default()
        };

        // When
        consumer.consume_game_event(
            &GameState::default(),
            &GameEvent::CellsRevealed {
                selection: CellSelection::Some(vec![v2(1, 2), v2(3, 2)]),
                by: "",
            },
        );
        let state = processor.process(state, &Instruction::Step);

        // Then
        assert_eq!(
            state.instructions,
            vec![
                Instruction::TotalVisiblePositions(0),
                Instruction::GetTraffic(v2(2, 2))
            ]
        );

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_append_total_visible_population_instruction_for_some_cell_selection() {
        // Given
        let total_visible_positions = 404;
        let game = UpdateProcess::new(total_visible_positions);
        let mut processor = VisibilitySim::new(&game.tx());
        let mut consumer = processor.consumer();

        // When
        consumer.consume_game_event(
            &GameState::default(),
            &GameEvent::CellsRevealed {
                selection: CellSelection::Some(vec![v2(1, 2), v2(3, 2)]),
                by: "",
            },
        );
        let state = processor.process(State::default(), &Instruction::Step);

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::TotalVisiblePositions(total_visible_positions)]
        );

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_do_nothing_if_no_messages() {
        // Given
        let game = UpdateProcess::new(0);
        let mut processor = VisibilitySim::new(&game.tx());

        // When
        let state = processor.process(State::default(), &Instruction::Step);

        // Then
        assert_eq!(state.instructions, vec![]);

        // Finally
        game.shutdown();
    }
}
