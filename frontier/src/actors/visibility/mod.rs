use crate::game::{Game, GameEvent};
use crate::visibility_computer::VisibilityComputer;
use commons::async_channel::{Receiver, RecvError};
use commons::async_update::{
    update_channel,
    Process,
    UpdateReceiver,
    UpdateSender as AsyncUpdateSender, // TODO remove aliasing after removing other type
};
use commons::futures::future::FutureExt;
use commons::grid::Grid;
use commons::update::UpdateSender;
use commons::{v2, M, V2};
use isometric::cell_traits::WithElevation;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const HANDLE: &str = "world_artist_actor";

pub struct Visibility {
    rx: UpdateReceiver<Visibility>,
    tx: AsyncUpdateSender<Visibility>,
    game_rx: Receiver<GameEvent>,
    game_tx: UpdateSender<Game>,
    visibility_computer: VisibilityComputer,
    state: VisibilityHandlerState,
    grid: Option<M<Elevation>>,
    run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct VisibilityHandlerState {
    active: bool,
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
        let (tx, rx) = update_channel();
        Visibility {
            rx,
            tx,
            game_rx,
            game_tx: game_tx.clone_with_handle(HANDLE),
            visibility_computer: VisibilityComputer::default(),
            state: VisibilityHandlerState {
                active: true,
                visited: None,
            },
            grid: None,
            run: true,
        }
    }

    pub fn tx(&self) -> &AsyncUpdateSender<Visibility> {
        &self.tx
    }

    pub async fn run(&mut self) {
        while self.run {
            if self.grid.is_some() {
                select! {
                    mut update = self.rx.get_update().fuse() => update.process(self),
                    event = self.game_rx.recv().fuse() => self.handle_game_event(event).await
                }
            } else {
                select! {
                    event = self.game_rx.recv().fuse() => self.handle_game_event(event).await // TODO can we avoid the repeat?
                }
            }
        }
    }

    pub fn check_visibility_and_reveal(&mut self, visited: HashSet<V2<usize>>) {
        for cell in visited {
            self.check_visibility_and_reveal_one(cell);
        }
    }

    fn check_visibility_and_reveal_one(&mut self, cell: V2<usize>) {
        let visited = self.state.visited.as_mut().unwrap();
        let visited = unwrap_or!(visited.mut_cell(&cell), return); // TODO create function for visited stuff
        if *visited {
            return;
        } else {
            *visited = true;
        }
        let newly_visible = self
            .visibility_computer
            .get_visible_from(self.grid.as_ref().unwrap(), cell);

        self.game_tx.update(move |game: &mut Game| {
            game.reveal_cells(newly_visible.into_iter().collect(), HANDLE)
        });
    }

    async fn handle_game_event(&mut self, event: Result<GameEvent, RecvError>) {
        if let GameEvent::Init = event.unwrap() {
            self.init().await;
        }
    }

    async fn init(&mut self) {
        let (width, height) = self.game_tx.update(|game| get_dimensions(game)).await;
        self.state.visited = Some(M::from_element(width, height, false));
        self.grid = Some(self.game_tx.update(|game| get_elevations(game)).await);
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
