mod coloring;

use crate::artists::{Slab, WorldArtist};
use crate::game::Game;
use crate::game_event_consumers::world_coloring;
use commons::async_channel::{unbounded, Receiver, RecvError, Sender as AsyncSender};
use commons::futures::future::FutureExt;
use commons::log::debug;
use commons::update::UpdateSender;
use commons::V2;
use isometric::Command;
use isometric::Event;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;

const HANDLE: &str = "world_artist_actor";

pub enum RedrawType {
    Init,
    All,
    Tile(V2<usize>),
}

pub struct Redraw {
    pub redraw_type: RedrawType,
    pub when: u128,
}

pub struct WorldArtistActor {
    rx: Receiver<Redraw>,
    tx: AsyncSender<Redraw>,
    engine_rx: Receiver<Arc<Event>>,
    game_tx: UpdateSender<Game>,
    command_tx: Sender<Vec<Command>>,
    world_artist: WorldArtist,
    last_redraw: HashMap<V2<usize>, u128>,
    run: bool,
}

impl WorldArtistActor {
    pub fn new(
        engine_rx: Receiver<Arc<Event>>,
        game_tx: &UpdateSender<Game>,
        command_tx: Sender<Vec<Command>>,
        world_artist: WorldArtist,
    ) -> WorldArtistActor {
        let (tx, rx) = unbounded();
        WorldArtistActor {
            rx,
            tx,
            engine_rx,
            game_tx: game_tx.clone_with_handle(HANDLE),
            command_tx,
            last_redraw: hashmap! {},
            world_artist,
            run: true,
        }
    }

    pub fn tx(&self) -> &AsyncSender<Redraw> {
        &self.tx
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                event = self.rx.recv().fuse() => self.redraw(event).await,
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        let event: Arc<Event> = event.unwrap();
        if let Event::Shutdown = *event {
            self.shutdown().await
        }
    }

    async fn shutdown(&mut self) {
        self.run = false;
    }

    async fn redraw(&mut self, event: Result<Redraw, RecvError>) {
        let Redraw { redraw_type, when } = ok_or!(event, return);
        match redraw_type {
            RedrawType::Init => self.init(),
            RedrawType::All => self.redraw_all(when).await,
            RedrawType::Tile(tile) => self.redraw_tile(tile, when).await,
        };
    }

    fn init(&mut self) {
        let commands = self.world_artist.init();
        self.command_tx.send(commands).unwrap();
    }

    async fn redraw_all(&mut self, when: u128) {
        for slab in self.world_artist.get_all_slabs() {
            self.redraw_slab(slab, when).await;
        }
    }

    async fn redraw_tile(&mut self, tile: V2<usize>, when: u128) {
        let slab = Slab::new(tile, self.world_artist.params().slab_size); // TODO maybe move get_slab to world_artist?
        self.redraw_slab(slab, when).await;
    }

    async fn redraw_slab(&mut self, slab: Slab, when: u128) {
        debug!("Maybe redrawing {:?}@{}", slab.from, when);
        if self
            .last_redraw
            .get(&slab.from)
            .map(|last_redraw| when <= *last_redraw)
            .unwrap_or(false)
        {
            debug!(" - already redrawn @{}", self.last_redraw[&slab.from]);
            return;
        }
        let world_artist = self.world_artist.clone();
        let from = slab.from;
        let (game_micros, commands) = self
            .game_tx
            .update(move |game| draw_slab(&game, world_artist, slab))
            .await;
        self.last_redraw.insert(from, game_micros);
        self.command_tx.send(commands).unwrap();
        debug!(" - redrawn");
    }
}

fn draw_slab(game: &Game, world_artist: WorldArtist, slab: Slab) -> (u128, Vec<Command>) {
    // TODO fix double return
    let game_state = game.game_state();
    (
        game_state.game_micros,
        world_artist.draw_slab(&game_state.world, &world_coloring(game_state, true), &slab),
    ) // TODO support territory layer
}
