mod coloring;

pub use coloring::{BaseColors, WorldColoringParameters};
use commons::async_trait::async_trait;

use crate::artists::{HouseArtist, ResourceArtist, ResourceArtistParameters, Slab, WorldArtist};
use crate::nation::NationDescription;
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::has::HasParameters;
use crate::traits::{
    Micros, SendEngineCommands, WithResources, WithSettlements, WithTerritory, WithWorld,
};
use coloring::{world_coloring, Overlay};
use commons::{v2, M, V2};
use isometric::{Button, Color, ElementState, Event, VirtualKeyCode};
use std::collections::{HashMap, HashSet};
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
    cx: T,
    bindings: WorldArtistActorBindings,
    world_artist: WorldArtist,
    resource_artist: Option<ResourceArtist>,
    house_artist: HouseArtist,
    coloring_params: WorldColoringParameters,
    last_redraw: HashMap<V2<usize>, u128>,
    territory_layer: bool,
    nation_colors: HashMap<String, Color>,
}

impl<T> WorldArtistActor<T>
where
    T: HasParameters
        + Micros
        + SendEngineCommands
        + WithResources
        + WithSettlements
        + WithTerritory
        + WithWorld
        + Send
        + Sync,
{
    pub fn new(
        cx: T,
        world_artist: WorldArtist,
        house_artist: HouseArtist,
        coloring_params: WorldColoringParameters,
        overlay_alpha: f32,
        nation_descriptions: &[NationDescription],
    ) -> WorldArtistActor<T> {
        WorldArtistActor {
            cx,
            bindings: WorldArtistActorBindings::default(),
            last_redraw: hashmap! {},
            world_artist,
            resource_artist: None,
            house_artist,
            coloring_params,
            nation_colors: Self::get_nation_colors(nation_descriptions, overlay_alpha),
            territory_layer: false,
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

    pub async fn init(&mut self) {
        self.init_world_artist().await;
        self.init_resource_artist().await;
        self.redraw_all().await;
    }

    pub async fn init_world_artist(&self) {
        self.cx.send_engine_commands(self.world_artist.init()).await;
    }

    pub async fn init_resource_artist(&mut self) {
        self.resource_artist = self
            .cx
            .with_resources(|resources| {
                Some(ResourceArtist::new(
                    ResourceArtistParameters::default(),
                    &resources,
                ))
            })
            .await;
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

    pub async fn redraw_tiles_at(&mut self, tiles: HashSet<V2<usize>>, when: u128) {
        let slabs = tiles
            .into_iter()
            .map(|tile| Slab::at(tile, self.world_artist.params().slab_size))
            .collect::<HashSet<_>>();
        for slab in slabs {
            self.redraw_slab(slab, when).await;
        }
    }

    async fn when(&mut self) -> u128 {
        self.cx.micros().await
    }

    async fn redraw_slab(&mut self, slab: Slab, when: u128) {
        if self.has_been_redrawn_after(&slab, &when) {
            return;
        }

        let territory_colors = self.get_territory_colors(&slab).await;

        let generated_after = self.cx.micros().await;
        self.draw_slab_with_world_artist(&slab, &territory_colors)
            .await;
        self.draw_slab_with_resource_artist(&slab).await;
        self.draw_slab_with_house_artist(&slab, &territory_colors)
            .await;

        self.last_redraw.insert(slab.from, generated_after);
    }

    async fn get_territory_colors(&self, slab: &Slab) -> M<Option<Color>> {
        let territory = self.get_territory(slab).await;
        let nations = self.get_nations(&territory).await;

        territory.map(|settlement| {
            settlement
                .and_then(|settlement| nations.get(&settlement))
                .and_then(|nation| self.nation_colors.get(nation))
                .copied()
        })
    }

    async fn draw_slab_with_world_artist(
        &mut self,
        slab: &Slab,
        territory_colors: &M<Option<Color>>,
    ) {
        let overlay = self.get_territory_overlay(&slab, territory_colors);
        let commands = self
            .cx
            .with_world(|world| {
                self.world_artist.draw_slab(
                    &world,
                    &world_coloring(&world, &self.coloring_params, &overlay),
                    slab,
                )
            })
            .await;
        self.cx.send_engine_commands(commands).await;
    }

    fn get_territory_overlay<'a>(
        &self,
        slab: &Slab,
        territory_colors: &'a M<Option<Color>>,
    ) -> Option<Overlay<'a>> {
        if !self.territory_layer {
            None
        } else {
            Some(Overlay {
                from: slab.from,
                colors: territory_colors,
            })
        }
    }

    async fn draw_slab_with_resource_artist(&mut self, slab: &Slab) {
        let resource_artist = unwrap_or!(&self.resource_artist, return);
        let commands = self
            .cx
            .with_world(|world| resource_artist.draw(world, &slab.from, &slab.to()))
            .await;
        self.cx.send_engine_commands(commands).await;
    }

    async fn draw_slab_with_house_artist(
        &mut self,
        slab: &Slab,
        territory_colors: &M<Option<Color>>,
    ) {
        let commands = self
            .cx
            .with_world(|world| {
                self.house_artist
                    .draw(world, &slab.from, &slab.to(), territory_colors)
            })
            .await;
        self.cx.send_engine_commands(commands).await;
    }

    fn has_been_redrawn_after(&self, slab: &Slab, when: &u128) -> bool {
        self.last_redraw
            .get(&slab.from)
            .map(|last_redraw| when < last_redraw)
            .unwrap_or(false)
    }

    async fn get_territory(&self, slab: &Slab) -> M<Option<V2<usize>>> {
        self.cx
            .with_territory(|territory| {
                M::from_fn(slab.slab_size, slab.slab_size, |x, y| {
                    territory
                        .who_controls_tile(&v2(slab.from.x + x, slab.from.y + y))
                        .map(|claim| claim.controller)
                })
            })
            .await
    }

    async fn get_nations(&self, territory: &M<Option<V2<usize>>>) -> HashMap<V2<usize>, String> {
        let distinct = territory.iter().flatten().copied().collect::<HashSet<_>>();
        self.cx
            .with_settlements(|settlements| {
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

#[async_trait]
impl<T> HandleEngineEvent for WorldArtistActor<T>
where
    T: HasParameters
        + Micros
        + SendEngineCommands
        + WithResources
        + WithSettlements
        + WithTerritory
        + WithWorld
        + Send
        + Sync,
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
