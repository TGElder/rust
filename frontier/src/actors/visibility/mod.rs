use crate::game::{Game, GameEvent};
use crate::traits::{RevealPositions, SendGame, SendWorld};
use crate::visibility_computer::VisibilityComputer;
use crate::world::World;
use commons::async_channel::{Receiver, RecvError};
use commons::fn_sender::{FnMessage, FnMessageExt, FnReceiver};
use commons::futures::future::FutureExt;
use commons::grid::Grid;
use commons::{v2, M, V2};
use isometric::cell_traits::WithElevation;
use isometric::Event;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;

const NAME: &str = "world_artist_actor";

pub struct VisibilityActor<T> {
    x: T,
    rx: FnReceiver<VisibilityActor<T>>,
    engine_rx: Receiver<Arc<Event>>,
    game_rx: Receiver<GameEvent>,
    visibility_computer: VisibilityComputer,
    state: VisibilityHandlerState,
    elevations: Option<M<Elevation>>,
    run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct VisibilityHandlerState {
    visited: Option<M<bool>>,
    active: bool,
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

impl<T> VisibilityActor<T>
where
    T: RevealPositions + SendGame + SendWorld + Send,
{
    pub fn new(
        x: T,
        rx: FnReceiver<VisibilityActor<T>>,
        engine_rx: Receiver<Arc<Event>>,
        game_rx: Receiver<GameEvent>,
    ) -> VisibilityActor<T> {
        VisibilityActor {
            x,
            rx,
            engine_rx,
            game_rx,
            visibility_computer: VisibilityComputer::default(),
            state: VisibilityHandlerState {
                visited: None,
                active: true,
            },
            elevations: None,
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            if !self.state.active || self.elevations.is_some() {
                select! {
                    mut message = self.rx.get_message().fuse() => self.handle_message(message).await,
                    event = self.game_rx.recv().fuse() => self.handle_game_event(event).await,
                    event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await,
                }
            } else {
                select! {
                    event = self.game_rx.recv().fuse() => self.handle_game_event(event).await,
                    event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await,
                }
            }
        }
    }

    async fn handle_message(&mut self, mut message: FnMessage<VisibilityActor<T>>) {
        if self.state.active {
            message.apply(self).await;
        }
    }

    pub async fn check_visibility_and_reveal(&mut self, visited: HashSet<V2<usize>>) {
        for position in visited {
            self.check_visibility_and_reveal_for_position(position)
                .await;
        }
    }

    pub fn disable_visibility_computation(&mut self) {
        self.state.active = false;
    }

    async fn check_visibility_and_reveal_for_position(&mut self, position: V2<usize>) {
        let already_visited = ok_or!(self.already_visited(&position), return);
        if *already_visited {
            return;
        } else {
            self.set_visited(&position);
        }

        let visible = self
            .visibility_computer
            .get_visible_from(self.elevations.as_ref().unwrap(), position);

        self.x
            .reveal_positions(visible.iter().cloned().collect(), NAME)
            .await;
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
        match event.unwrap() {
            GameEvent::NewGame => self.new_game().await,
            GameEvent::Init => self.init().await,
            GameEvent::Load(path) => self.load(&path),
            GameEvent::Save(path) => self.save(&path),
            _ => (),
        }
    }

    async fn new_game(&mut self) {
        self.try_disable_visibility_computation().await;
        if self.state.active {
            self.init_visited().await;
        }
    }

    async fn try_disable_visibility_computation(&mut self) {
        if self.x.send_game(|game| get_reveal_all(game)).await {
            self.disable_visibility_computation();
        }
    }

    async fn init(&mut self) {
        self.init_elevations().await;
    }

    async fn init_visited(&mut self) {
        let (width, height) = self.x.send_world(|world| get_dimensions(world)).await;
        self.state.visited = Some(M::from_element(width, height, false));
    }

    async fn init_elevations(&mut self) {
        self.elevations = Some(self.x.send_world(|world| get_elevations(world)).await);
    }

    fn get_path(path: &str) -> String {
        format!("{}.visibility_actor", path)
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

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        if let Event::Shutdown = *event.unwrap() {
            self.shutdown();
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }
}

fn get_reveal_all(game: &Game) -> bool {
    game.game_state().params.reveal_all
}

fn get_dimensions(world: &World) -> (usize, usize) {
    (world.width(), world.height())
}

fn get_elevations(world: &World) -> M<Elevation> {
    let sea_level = world.sea_level();
    M::from_fn(world.width(), world.height(), |x, y| Elevation {
        elevation: world.get_cell_unsafe(&v2(x, y)).elevation.max(sea_level),
    })
}
