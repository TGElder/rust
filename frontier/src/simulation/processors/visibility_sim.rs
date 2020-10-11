use super::*;
use crate::game::traits::VisiblePositions;
use crate::game::{CaptureEvent, CellSelection, GameEvent, GameEventConsumer, GameState};
use commons::grid::Grid;
use isometric::Event;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum VisibilityMessage {
    Position(V2<usize>),
    All,
}

pub struct VisibilitySim<G>
where
    G: VisiblePositions,
{
    tx: Sender<VisibilityMessage>,
    rx: Receiver<VisibilityMessage>,
    game: UpdateSender<G>,
}

#[async_trait]
impl<G> Processor for VisibilitySim<G>
where
    G: VisiblePositions,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Step => (),
            _ => return state,
        };
        let messages = self.get_messages();
        if !messages.is_empty() {
            state.instructions.push(Instruction::VisibleLandPositions(
                self.visible_land_positions().await,
            ));
        }
        let positions_to_refresh = get_traffic_positions(&state, &messages);
        if !positions_to_refresh.is_empty() {
            state
                .instructions
                .push(Instruction::RefreshPositions(positions_to_refresh));
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

    async fn visible_land_positions(&mut self) -> usize {
        self.game.update(|game| visible_land_positions(game)).await
    }
}

fn visible_land_positions<G>(game: &mut G) -> usize
where
    G: VisiblePositions,
{
    game.visible_land_positions()
}

fn get_traffic_positions(state: &State, messages: &[VisibilityMessage]) -> HashSet<V2<usize>> {
    messages
        .iter()
        .flat_map(|message| get_position(message))
        .flat_map(|position| state.traffic.expand_position(position))
        .filter(|position| should_get_traffic(state, position))
        .collect()
}

fn get_position(message: &VisibilityMessage) -> Option<&V2<usize>> {
    match message {
        VisibilityMessage::Position(position) => Some(position),
        VisibilityMessage::All => None,
    }
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

    fn send_messages_for_cells(&mut self, cells: &[V2<usize>]) {
        for cell in cells {
            self.send_message_for_cell(*cell);
        }
    }

    fn send_message_for_cell(&mut self, cell: V2<usize>) {
        self.tx.send(VisibilityMessage::Position(cell)).unwrap();
    }

    fn send_message_for_all(&mut self) {
        self.tx.send(VisibilityMessage::All).unwrap();
    }
}

impl GameEventConsumer for VisibilitySimConsumer {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::CellsRevealed { selection, .. } = event {
            match selection {
                CellSelection::Some(cells) => self.send_messages_for_cells(cells),
                CellSelection::All => self.send_message_for_all(),
            }
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

    use crate::resource::Resource;
    use crate::route::RouteKey;
    use commons::futures::executor::block_on;
    use commons::grid::Grid;
    use commons::same_elements;
    use commons::update::UpdateProcess;
    use commons::v2;
    use std::collections::HashSet;

    #[test]
    fn should_refresh_positions_surrounding_revealed_cell_with_traffic() {
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
        let state = block_on(processor.process(state, &Instruction::Step));

        // Then
        assert!(same_elements(
            &state.instructions,
            &[
                Instruction::VisibleLandPositions(0),
                Instruction::RefreshPositions(hashset! {
                    v2(0, 1),
                    v2(1, 1),
                    v2(2, 1),
                    v2(0, 2),
                    v2(1, 2),
                    v2(2, 2),
                    v2(0, 3),
                    v2(1, 3),
                    v2(2, 3)
                })
            ]
        ));

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_not_refresh_positions_surrounding_revealed_cell_without_traffic() {
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
        let state = block_on(processor.process(state, &Instruction::Step));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::VisibleLandPositions(0)]
        );

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_append_visible_land_positions_instruction_for_some_cell_selection() {
        // Given
        let visible_land_positions = 404;
        let game = UpdateProcess::new(visible_land_positions);
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
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::VisibleLandPositions(visible_land_positions)]
        );

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_append_visible_land_positions_instruction_for_all_cell_selection() {
        // Given
        let visible_land_positions = 404;
        let game = UpdateProcess::new(visible_land_positions);
        let mut processor = VisibilitySim::new(&game.tx());
        let mut consumer = processor.consumer();

        // When
        consumer.consume_game_event(
            &GameState::default(),
            &GameEvent::CellsRevealed {
                selection: CellSelection::All,
                by: "",
            },
        );
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::VisibleLandPositions(visible_land_positions)]
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
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert_eq!(state.instructions, vec![]);

        // Finally
        game.shutdown();
    }
}
