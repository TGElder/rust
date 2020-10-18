mod coloring;

use commons::async_channel::{Receiver, RecvError, Sender as AsyncSender};
use commons::futures::future::FutureExt;
use commons::V2;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Instant;
use crate::artists::{WorldArtist, Slab};
use commons::update::UpdateSender;
use crate::game::Game;
use isometric::Command;
use crate::game_event_consumers::world_coloring;

pub struct Redraw{
    tile: V2<usize>,
    when: u128,
}

pub struct WorldArtistActor{
    rx: Receiver<Redraw>,
    tx: AsyncSender<Redraw>,
    game_tx: UpdateSender<Game>,
    command_tx: Sender<Vec<Command>>,
    world_artist: WorldArtist,
    last_redraw: HashMap<V2<usize>, u128>,
    run: bool,
}

impl WorldArtistActor{

    pub async fn run(&mut self) {
        while self.run {
            select! {
                event = self.rx.recv().fuse() => self.redraw(event).await
            }
        }
    }

    async fn redraw(&mut self, event: Result<Redraw, RecvError>) {
        let Redraw{tile, when} = ok_or!(event, return);
        let slab = Slab::new(tile, 64);
        if self.last_redraw.get(&tile).map(|last_redraw| when <= *last_redraw).unwrap_or(false) {
            return;
        }
        let world_artist = self.world_artist.clone();
        let commands = self.game_tx.update(move |game| draw_slab(&game, world_artist, slab)).await;
        self.command_tx.send(commands);
    }
}

fn draw_slab(game: &Game, world_artist: WorldArtist, slab: Slab) -> Vec<Command> {
    let game_state = game.game_state();
    world_artist.draw_slab(&game_state.world, &world_coloring(game_state, false), &slab) // TODO support territory layer
}