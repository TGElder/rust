mod coloring;

pub use coloring::WorldColoringParameters;

use crate::artists::{Slab, WorldArtist};
use crate::game::{Game, GameEvent};
use coloring::world_coloring;
use commons::async_channel::{unbounded, Receiver, RecvError, Sender as AsyncSender};
use commons::fn_sender::FnSender;
use commons::futures::future::FutureExt;
use commons::V2;
use isometric::{Button, Command, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;

const HANDLE: &str = "world_artist_actor";

pub enum RedrawType {
    All,
    Tile(V2<usize>),
}

pub struct Redraw {
    pub redraw_type: RedrawType,
    pub when: u128,
}

pub struct WorldArtistActorBindings {
    toggle_territory_layer: Button,
}

impl Default for WorldArtistActorBindings {
    fn default() -> WorldArtistActorBindings {
        WorldArtistActorBindings {
            toggle_territory_layer: Button::Key(VirtualKeyCode::O),
        }
    }
}

pub struct WorldArtistActor {
    rx: Receiver<Redraw>,
    tx: AsyncSender<Redraw>,
    engine_rx: Receiver<Arc<Event>>,
    game_rx: Receiver<GameEvent>,
    game_tx: FnSender<Game>,
    command_tx: Sender<Vec<Command>>,
    bindings: WorldArtistActorBindings,
    world_artist: WorldArtist,
    last_redraw: HashMap<V2<usize>, u128>,
    run: bool,
    territory_layer: bool,
}

impl WorldArtistActor {
    pub fn new(
        engine_rx: Receiver<Arc<Event>>,
        game_rx: Receiver<GameEvent>,
        game_tx: &FnSender<Game>,
        command_tx: Sender<Vec<Command>>,
        world_artist: WorldArtist,
    ) -> WorldArtistActor {
        let (tx, rx) = unbounded();
        WorldArtistActor {
            rx,
            tx,
            engine_rx,
            game_rx,
            game_tx: game_tx.clone_with_name(HANDLE),
            command_tx,
            bindings: WorldArtistActorBindings::default(),
            last_redraw: hashmap! {},
            world_artist,
            run: true,
            territory_layer: false,
        }
    }

    pub fn tx(&self) -> &AsyncSender<Redraw> {
        &self.tx
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                event = self.rx.recv().fuse() => self.redraw(event).await,
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await,
                event = self.game_rx.recv().fuse() => self.handle_game_event(event).await
            }
        }
    }

    async fn redraw(&mut self, event: Result<Redraw, RecvError>) {
        let Redraw { redraw_type, when } = ok_or!(event, return);
        match redraw_type {
            RedrawType::All => self.redraw_all().await,
            RedrawType::Tile(tile) => self.redraw_tile(tile, when).await,
        };
    }

    async fn redraw_all(&mut self) {
        let when = self.when().await;
        for slab in self.world_artist.get_all_slabs() {
            self.redraw_slab(slab, when).await;
        }
    }

    async fn when(&mut self) -> u128 {
        self.game_tx
            .send(|game| game.game_state().game_micros)
            .await
    }

    async fn redraw_tile(&mut self, tile: V2<usize>, when: u128) {
        let slab = Slab::at(tile, self.world_artist.params().slab_size);
        self.redraw_slab(slab, when).await;
    }

    async fn redraw_slab(&mut self, slab: Slab, when: u128) {
        if self.has_been_redrawn_after(&slab, &when) {
            return;
        }

        let world_artist = self.world_artist.clone();
        let territory_layer = self.territory_layer;
        let Commands {
            generated_at,
            commands,
        } = self
            .game_tx
            .send(move |game| draw_slab(&game, world_artist, slab, territory_layer))
            .await;

        self.last_redraw.insert(slab.from, generated_at);
        self.command_tx.send(commands).unwrap();
    }

    fn has_been_redrawn_after(&self, slab: &Slab, when: &u128) -> bool {
        self.last_redraw
            .get(&slab.from)
            .map(|last_redraw| when < last_redraw)
            .unwrap_or(false)
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        match *event.unwrap() {
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers: ModifiersState { alt: false, .. },
                ..
            } if *button == self.bindings.toggle_territory_layer => {
                self.toggle_territory_layer().await
            }
            Event::Shutdown => self.shutdown().await,
            _ => (),
        }
    }

    async fn toggle_territory_layer(&mut self) {
        self.territory_layer = !self.territory_layer;
        self.redraw_all().await;
    }

    async fn shutdown(&mut self) {
        self.run = false;
    }

    async fn handle_game_event(&mut self, event: Result<GameEvent, RecvError>) {
        if let GameEvent::Init = event.unwrap() {
            self.init();
            self.redraw_all().await;
        }
    }

    fn init(&mut self) {
        let commands = self.world_artist.init();
        self.command_tx.send(commands).unwrap();
    }
}

struct Commands {
    generated_at: u128,
    commands: Vec<Command>,
}

fn draw_slab(
    game: &Game,
    world_artist: WorldArtist,
    slab: Slab,
    territory_layer: bool,
) -> Commands {
    let game_state = game.game_state();
    Commands {
        generated_at: game_state.game_micros,
        commands: world_artist.draw_slab(
            &game_state.world,
            &world_coloring(game_state, territory_layer),
            &slab,
        ),
    }
}
