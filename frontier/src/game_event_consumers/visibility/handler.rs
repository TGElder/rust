use crate::game::*;
use crate::visibility_computer::*;
use commons::grid::Grid;
use commons::update::*;
use commons::M;
use commons::V2;
use isometric::Event;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

const HANDLE: &str = "visibility_handler";

pub struct VisibilityHandler {
    game_tx: UpdateSender<Game>,
    tx: Sender<VisibilityHandlerMessage>,
    rx: Receiver<VisibilityHandlerMessage>,
    visibility_computer: VisibilityComputer,
    state: VisibilityHandlerState,
}

pub struct VisibilityHandlerMessage {
    pub visited: HashSet<V2<usize>>,
}

#[derive(Serialize, Deserialize)]
pub struct VisibilityHandlerState {
    active: bool,
    visibility_queue: VecDeque<V2<usize>>,
    visited: Option<M<bool>>,
}

impl VisibilityHandler {
    pub fn new(game_tx: &UpdateSender<Game>) -> VisibilityHandler {
        let (tx, rx) = channel();
        VisibilityHandler {
            tx,
            rx,
            game_tx: game_tx.clone_with_handle(HANDLE),
            visibility_computer: VisibilityComputer::default(),
            state: VisibilityHandlerState {
                active: true,
                visited: None,
                visibility_queue: VecDeque::new(),
            },
        }
    }

    pub fn tx(&self) -> &Sender<VisibilityHandlerMessage> {
        &self.tx
    }

    fn new_game(&mut self, game_state: &GameState) {
        let world = &game_state.world;
        self.state.visited = Some(M::from_element(world.width(), world.height(), false));
    }

    fn tick(&mut self, game_state: &GameState) {
        self.read_messages();
        self.do_one_check(game_state);
    }

    fn read_messages(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            for position in self.update_visited_get_newly_visited(message.visited) {
                self.state.visibility_queue.push_back(position);
            }
        }
    }

    fn drain_messages(&mut self) {
        while let Ok(_) = self.rx.try_recv() {
        }
    }

    fn update_visited_get_newly_visited(
        &mut self,
        mut cells: HashSet<V2<usize>>,
    ) -> HashSet<V2<usize>> {
        let visited = unwrap_or!(&mut self.state.visited, return HashSet::new());
        let newly_visited: HashSet<V2<usize>> = cells
            .drain()
            .filter(|cell| !visited.get_cell_unsafe(cell))
            .collect();
        newly_visited
            .iter()
            .for_each(|cell| *visited.mut_cell_unsafe(cell) = true);
        newly_visited
    }

    fn do_one_check(&mut self, game_state: &GameState) {
        if let Some(position) = self.state.visibility_queue.pop_front() {
            self.check_visibility_and_reveal(game_state, position)
        }
    }

    fn check_visibility_and_reveal(&mut self, game_state: &GameState, cell: V2<usize>) {
        let newly_visible = self
            .visibility_computer
            .get_newly_visible_from(&game_state.world, cell);

        self.game_tx
            .update(move |game: &mut Game| game.reveal_cells(newly_visible));
    }

    fn get_path(path: &str) -> String {
        format!("{}.visibility_handler", path)
    }

    fn save(&mut self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
    }

    fn deactive(&mut self) {
        self.state.active = false;
    }
}

impl GameEventConsumer for VisibilityHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if !self.state.active {
            self.drain_messages();
            return CaptureEvent::No;
        }
        match event {
            GameEvent::NewGame => self.new_game(game_state),
            GameEvent::Tick { .. } => self.tick(game_state),
            GameEvent::CellsRevealed(CellSelection::All) => self.deactive(),
            GameEvent::Save(path) => self.save(&path),
            GameEvent::Load(path) => self.load(&path),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
