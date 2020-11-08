use crate::game::{Game, GameEvent};
use crate::visibility_computer::VisibilityComputer;
use commons::actor::{action_channel, Act, Actor, Director};
use commons::async_channel::{Receiver, RecvError};
use commons::futures::future::FutureExt;
use commons::grid::Grid;
use commons::update::UpdateSender;
use commons::{v2, M, V2};
use isometric::cell_traits::WithElevation;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const HANDLE: &str = "world_artist_actor";

pub struct Visibility {
    tx: Director<Visibility>,
    rx: Actor<Visibility>,
    game_rx: Receiver<GameEvent>,
    game_tx: UpdateSender<Game>,
    visibility_computer: VisibilityComputer,
    state: VisibilityHandlerState,
    elevations: Option<M<Elevation>>,
    run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct VisibilityHandlerState {
    visited: Option<M<bool>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Elevation {
    elevation: f32,
}

impl WithElevation for Elevation {
    fn elevation(&self) -> f32 {
        self.elevation
    }
}

impl Visibility {
    pub fn new(game_rx: Receiver<GameEvent>, game_tx: &UpdateSender<Game>) -> Visibility {
        let (tx, rx) = action_channel();
        Visibility {
            tx,
            rx,
            game_rx,
            game_tx: game_tx.clone_with_handle(HANDLE),
            visibility_computer: VisibilityComputer::default(),
            state: VisibilityHandlerState { visited: None },
            elevations: None,
            run: true,
        }
    }

    pub fn tx(&self) -> &Director<Visibility> {
        &self.tx
    }

    pub async fn run(&mut self) {
        while self.run {
            if self.elevations.is_some() {
                select! {
                    mut update = self.rx.get_update().fuse() => update.act(self),
                    event = self.game_rx.recv().fuse() => self.handle_game_event(event).await
                }
            } else {
                select! {
                    event = self.game_rx.recv().fuse() => self.handle_game_event(event).await
                }
            }
        }
    }

    pub fn check_visibility_and_reveal(&mut self, visited: HashSet<V2<usize>>) {
        for position in visited {
            self.check_visibility_and_reveal_position(position);
        }
    }

    fn check_visibility_and_reveal_position(&mut self, position: V2<usize>) {
        let already_visited = ok_or!(self.already_visited(&position), return);
        if *already_visited {
            return;
        } else {
            self.set_visited(&position);
        }

        let visible = self
            .visibility_computer
            .get_visible_from(self.elevations.as_ref().unwrap(), position);

        self.game_tx.update(move |game: &mut Game| {
            game.reveal_cells(visible.into_iter().collect(), HANDLE)
        });
    }

    fn already_visited(&self, position: &V2<usize>) -> Result<&bool, ()> {
        let visited = self.state.visited.as_ref().unwrap();
        visited.get_cell(&position).ok_or(())
    }

    fn set_visited(&mut self, position: &V2<usize>) {
        let visited = self.state.visited.as_mut().unwrap();
        if let Some(visited) = visited.mut_cell(&position) {
            *visited = true;
        }
    }

    async fn handle_game_event(&mut self, event: Result<GameEvent, RecvError>) {
        if let GameEvent::Init = event.unwrap() {
            self.init().await;
        }
    }

    async fn init(&mut self) {
        self.init_visited().await;
        self.init_elevations().await;
    }

    async fn init_visited(&mut self) {
        let (width, height) = self.game_tx.update(|game| get_dimensions(game)).await;
        self.state.visited = Some(M::from_element(width, height, false));
    }

    async fn init_elevations(&mut self) {
        self.elevations = Some(self.game_tx.update(|game| get_elevations(game)).await);
    }
}

fn get_dimensions(game: &Game) -> (usize, usize) {
    let world = &game.game_state().world;
    (world.width(), world.height())
}

fn get_elevations(game: &Game) -> M<Elevation> {
    let world = &game.game_state().world;
    M::from_fn(world.width(), world.height(), |x, y| Elevation {
        elevation: game.game_state().world.get_cell_unsafe(&v2(x, y)).elevation,
    })
}
