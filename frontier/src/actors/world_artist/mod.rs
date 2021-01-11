mod coloring;

pub use coloring::WorldColoringParameters;
use commons::async_trait::async_trait;

use crate::artists::{Slab, WorldArtist};
use crate::game::Game;
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{Micros, SendGame};
use coloring::world_coloring;
use commons::V2;
use isometric::{Button, Command, ElementState, Event, VirtualKeyCode};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;

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

pub struct WorldArtistActor<T> {
    tx: T,
    command_tx: Sender<Vec<Command>>,
    bindings: WorldArtistActorBindings,
    world_artist: WorldArtist,
    last_redraw: HashMap<V2<usize>, u128>,
    territory_layer: bool,
}

impl<T> WorldArtistActor<T>
where
    T: Micros + SendGame + Send,
{
    pub fn new(
        tx: T,
        command_tx: Sender<Vec<Command>>,
        world_artist: WorldArtist,
    ) -> WorldArtistActor<T> {
        WorldArtistActor {
            tx,
            command_tx,
            bindings: WorldArtistActorBindings::default(),
            last_redraw: hashmap! {},
            world_artist,
            territory_layer: false,
        }
    }

    pub async fn init(&mut self) {
        let commands = self.world_artist.init();
        self.command_tx.send(commands).unwrap();
        self.redraw_all().await;
    }

    async fn redraw_all(&mut self) {
        let when = self.when().await;
        self.redraw_all_at(when).await;
    }

    pub async fn redraw_all_at(&mut self, when: u128) {
        for slab in self.world_artist.get_all_slabs() {
            self.redraw_slab(slab, when).await;
        }
    }

    pub async fn redraw_tile_at(&mut self, tile: V2<usize>, when: u128) {
        let slab = Slab::at(tile, self.world_artist.params().slab_size);
        self.redraw_slab(slab, when).await;
    }

    async fn when(&mut self) -> u128 {
        self.tx.micros().await
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
            .tx
            .send_game(move |game| draw_slab(&game, world_artist, slab, territory_layer))
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

    async fn toggle_territory_layer(&mut self) {
        self.territory_layer = !self.territory_layer;
        self.redraw_all().await;
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

#[async_trait]
impl<T> HandleEngineEvent for WorldArtistActor<T>
where
    T: Micros + SendGame + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        match *event {
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers,
                ..
            } if *button == self.bindings.toggle_territory_layer
                && !modifiers.alt()
                && modifiers.ctrl() =>
            {
                self.toggle_territory_layer().await
            }
            _ => (),
        }
        Capture::No
    }
}
