
use crate::visibility_computer::VisibilityComputer;
use crate::game::{Game, GameEvent};
use crate::world::World;
use commons::async_channel::{unbounded, Receiver, RecvError, Sender as AsyncSender};
use commons::futures::future::FutureExt;
use commons::update::UpdateSender;
use commons::{M, V2};
use isometric::{Button, Command, ElementState, Event, ModifiersState, VirtualKeyCode};
use serde::{Deserialize, Serialize};
use std::collections::{VecDeque, HashSet};
use std::sync::mpsc::Sender;
use std::sync::Arc;

const HANDLE: &str = "world_artist_actor";

pub struct Visibility {
    rx: Receiver<VisibilityHandlerMessage>,
    tx: AsyncSender<VisibilityHandlerMessage>,
    game_rx: Receiver<GameEvent>,
    game_tx: UpdateSender<Game>,
    visibility_computer: VisibilityComputer,
    state: VisibilityHandlerState,
    world: World,
    run: bool,
}

pub struct VisibilityHandlerMessage {
    pub visited: HashSet<V2<usize>>, // TODO should this still be a HashSet?
}

#[derive(Serialize, Deserialize)]
pub struct VisibilityHandlerState {
    active: bool,
    visibility_queue: VecDeque<V2<usize>>,
    visited: Option<M<bool>>,
}

impl Visibility {
    pub fn new(
        game_rx: Receiver<GameEvent>,
        game_tx: &UpdateSender<Game>,
        world: World,
    ) -> Visibility {
        let (tx, rx) = unbounded();
        Visibility {
            rx,
            tx,
            game_rx,
            game_tx: game_tx.clone_with_handle(HANDLE),
            visibility_computer: VisibilityComputer::default(),
            state: VisibilityHandlerState {
                active: true,
                visited: None,
                visibility_queue: VecDeque::new(),
            },
            world,
            run: true,
        }
    }

    pub fn tx(&self) -> &AsyncSender<VisibilityHandlerMessage> {
        &self.tx
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                message = self.rx.recv().fuse() => self.handle_visibility_message(message),
                event = self.game_rx.recv().fuse() => self.handle_game_event(event).await
            }
        }
    }

    fn handle_visibility_message(&mut self, message: Result<VisibilityHandlerMessage, RecvError>) {
        let VisibilityHandlerMessage{visited} = message.unwrap();
        for cell in visited {
            self.check_visibility_and_reveal(cell);
        }
    }

    fn check_visibility_and_reveal(&mut self, cell: V2<usize>) {
        let newly_visible = self
            .visibility_computer
            .get_newly_visible_from(&self.world, cell);

        self.game_tx
            .update(move |game: &mut Game| game.reveal_cells(newly_visible, HANDLE));
    }


    async fn handle_game_event(&mut self, event: Result<GameEvent, RecvError>) {
        if let GameEvent::Init = event.unwrap() {
            self.init();
            
        }
    }

    fn init(&mut self) {
        
    }
}