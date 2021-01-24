mod coloring;

pub use coloring::{BaseColors, WorldColoringParameters};
use commons::async_trait::async_trait;

use crate::artists::{Slab, WorldArtist};
use crate::nation::NationDescription;
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{Micros, SendSettlements, SendTerritory, SendWorld};
use crate::world::World;
use coloring::{world_coloring, SlabOverlay};
use commons::{v2, M, V2};
use isometric::{Button, Color, Command, ElementState, Event, VirtualKeyCode};
use std::collections::{HashMap, HashSet};
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
    coloring_params: WorldColoringParameters,
    last_redraw: HashMap<V2<usize>, u128>,
    territory_layer: bool,
    nation_colors: HashMap<String, Color>,
}

impl<T> WorldArtistActor<T>
where
    T: Micros + SendSettlements + SendTerritory + SendWorld + Send,
{
    pub fn new(
        tx: T,
        command_tx: Sender<Vec<Command>>,
        world_artist: WorldArtist,
        coloring_params: WorldColoringParameters,
        overlay_alpha: f32,
        nation_descriptions: &[NationDescription],
    ) -> WorldArtistActor<T> {
        WorldArtistActor {
            tx,
            command_tx,
            bindings: WorldArtistActorBindings::default(),
            last_redraw: hashmap! {},
            world_artist,
            coloring_params,
            nation_colors: get_nation_colors(nation_descriptions, overlay_alpha),
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

        let generated_after = self.tx.micros().await;
        self.draw_slab(slab).await;

        self.last_redraw.insert(slab.from, generated_after);
    }

    async fn draw_slab(&mut self, slab: Slab) {
        let world_artist = self.world_artist.clone();
        let params = self.coloring_params;
        let overlay = self.territory_overlay(&slab).await;
        let commands = self
            .tx
            .send_world(move |world| {
                world_artist.draw_slab(&world, &world_coloring(&world, &params, &overlay), &slab)
            })
            .await;
        self.command_tx.send(commands).unwrap();
    }

    fn has_been_redrawn_after(&self, slab: &Slab, when: &u128) -> bool {
        self.last_redraw
            .get(&slab.from)
            .map(|last_redraw| when < last_redraw)
            .unwrap_or(false)
    }

    async fn territory_overlay(&mut self, slab: &Slab) -> Option<SlabOverlay> {
        if !self.territory_layer {
            None
        } else {
            Some(SlabOverlay {
                from: slab.from,
                colors: self.get_colors(*slab).await,
            })
        }
    }

    async fn get_colors(&mut self, slab: Slab) -> M<Option<Color>> {
        let territory = self.get_territory(slab).await;
        let nations = self.get_nations(&territory).await;

        territory.map(|settlement| {
            settlement
                .and_then(|settlement| nations.get(&settlement))
                .and_then(|nation| self.nation_colors.get(nation))
                .copied()
        })
    }

    async fn get_territory(&mut self, slab: Slab) -> M<Option<V2<usize>>> {
        self.tx
            .send_territory(move |territory| {
                M::from_fn(slab.slab_size, slab.slab_size, |x, y| {
                    territory
                        .who_controls_tile(&v2(slab.from.x + x, slab.from.y + y))
                        .map(|claim| claim.controller)
                })
            })
            .await
    }

    async fn get_nations(
        &mut self,
        territory: &M<Option<V2<usize>>>,
    ) -> HashMap<V2<usize>, String> {
        let distinct = territory.iter().flatten().copied().collect::<HashSet<_>>();
        self.tx
            .send_settlements(move |settlements| {
                distinct
                    .iter()
                    .flat_map(|settlement| settlements.get(settlement))
                    .map(|settlement| (settlement.position, settlement.nation.clone()))
                    .collect()
            })
            .await
    }

    async fn toggle_territory_layer(&mut self) {
        self.territory_layer = !self.territory_layer;
        self.redraw_all().await;
    }
}

fn get_nation_colors(
    nation_descriptions: &[NationDescription],
    overlay_alpha: f32,
) -> HashMap<String, Color> {
    nation_descriptions
        .iter()
        .map(|description| {
            (
                description.name.clone(),
                description.colors.primary.with_alpha(overlay_alpha),
            )
        })
        .collect()
}

#[async_trait]
impl<T> HandleEngineEvent for WorldArtistActor<T>
where
    T: Micros + SendSettlements + SendTerritory + SendWorld + Send + Sync,
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
