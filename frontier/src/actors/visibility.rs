use crate::traits::{RevealPositions, SendParameters, SendWorld};
use crate::visibility_computer::VisibilityComputer;
use crate::world::World;
use commons::grid::Grid;
use commons::{v2, M, V2};
use isometric::cell_traits::WithElevation;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};

const NAME: &str = "world_artist_actor";

pub struct VisibilityActor<T> {
    tx: T,
    visibility_computer: VisibilityComputer,
    state: VisibilityActorState,
    elevations: Option<M<Elevation>>,
}

#[derive(Serialize, Deserialize)]
pub struct VisibilityActorState {
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
    T: RevealPositions + SendParameters + SendWorld + Send,
{
    pub fn new(tx: T) -> VisibilityActor<T> {
        VisibilityActor {
            tx,
            visibility_computer: VisibilityComputer::default(),
            state: VisibilityActorState {
                visited: None,
                active: true,
            },
            elevations: None,
        }
    }

    pub async fn new_game(&mut self) {
        self.try_disable_visibility_computation().await;
        if self.state.active {
            self.init_visited().await;
        }
    }

    async fn try_disable_visibility_computation(&mut self) {
        if self.tx.send_parameters(|params| params.reveal_all).await {
            self.disable_visibility_computation();
        }
    }

    pub async fn init_visited(&mut self) {
        let (width, height) = self.tx.send_world(|world| get_dimensions(world)).await;
        self.state.visited = Some(M::from_element(width, height, false));
    }

    pub async fn init(&mut self) {
        self.init_elevations().await;
    }

    async fn init_elevations(&mut self) {
        self.elevations = Some(self.tx.send_world(|world| get_elevations(world)).await);
    }

    pub async fn check_visibility_and_reveal(&mut self, visited: HashSet<V2<usize>>) {
        if !self.state.active || self.elevations.is_none() {
            return;
        }
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

        self.tx.reveal_positions(visible, NAME).await;
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

    pub fn save(&self, path: &str) {
        let path = get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
    }
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

fn get_path(path: &str) -> String {
    format!("{}.visibility_actor", path)
}
