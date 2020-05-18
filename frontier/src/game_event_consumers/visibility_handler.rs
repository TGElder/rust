use super::*;

use crate::visibility_computer::*;
use commons::M;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::iter::{empty, once};

const HANDLE: &str = "visibility_handler";

pub struct VisibilityHandler {
    game_tx: UpdateSender<Game>,
    visibility_computer: VisibilityComputer,
    state: VisibilityHandlerState,
}

#[derive(Serialize, Deserialize)]
pub struct VisibilityHandlerState {
    active: bool,
    visited: Option<M<bool>>,
}

impl VisibilityHandler {
    pub fn new(game_tx: &UpdateSender<Game>) -> VisibilityHandler {
        VisibilityHandler {
            game_tx: game_tx.clone_with_handle(HANDLE),
            visibility_computer: VisibilityComputer::default(),
            state: VisibilityHandlerState {
                active: true,
                visited: None,
            },
        }
    }

    fn init(&mut self, game_state: &GameState) {
        let world = &game_state.world;
        if self.state.visited.is_none() {
            self.state.visited = Some(M::from_element(world.width(), world.height(), false));
        }
    }

    fn tick(&mut self, game_state: &GameState, from: &u128, to: &u128) {
        let visited_cells = avatar_visited_cells(game_state, from, to)
            .chain(town_visited_cells(game_state))
            .collect();
        let newly_visited = self.update_visited_get_newly_visited(visited_cells);
        self.check_visibility(game_state, newly_visited);
    }

    pub fn update_visited_get_newly_visited(
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

    fn check_visibility(&mut self, game_state: &GameState, cells: HashSet<V2<usize>>) {
        let mut newly_visible = vec![];
        for cell in cells {
            newly_visible.append(
                &mut self
                    .visibility_computer
                    .get_newly_visible_from(&game_state.world, cell),
            );
        }

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

fn avatar_visited_cells<'a>(
    game_state: &'a GameState,
    from: &'a u128,
    to: &'a u128,
) -> Box<dyn Iterator<Item = V2<usize>> + 'a> {
    if let Some(avatar) = game_state.selected_avatar() {
        match &avatar.state {
            AvatarState::Walking(path) => {
                let edges = path.edges_between_times(from, to);
                return Box::new(edges.into_iter().map(|edge| *edge.to()));
            }
            AvatarState::Stationary { position, .. } => return Box::new(once(*position)),
            _ => (),
        }
    }
    Box::new(empty())
}

fn town_visited_cells<'a>(game_state: &'a GameState) -> impl Iterator<Item = V2<usize>> + 'a {
    let world = &game_state.world;
    game_state
        .settlements
        .keys()
        .flat_map(move |town| world.get_corners_in_bounds(town))
}

impl GameEventConsumer for VisibilityHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if !self.state.active {
            return CaptureEvent::No;
        }
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::Tick {
                from_micros,
                to_micros,
            } => self.tick(game_state, from_micros, to_micros),
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
