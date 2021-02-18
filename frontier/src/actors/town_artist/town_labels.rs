use std::sync::Arc;

use crate::actors::town_artist::get_house_height_with_roof;
use crate::actors::TownArtistParameters;
use crate::settlement::*;

use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{
    GetNationDescription, GetSettlement, SendEngineCommands, Settlements, WithWorld,
};
use crate::world::World;
use commons::async_trait::async_trait;
use commons::{unsafe_ordering, V2};
use isometric::coords::WorldCoord;
use isometric::drawing::{draw_label, get_house_base_corners};
use isometric::{Button, Command, ElementState, Event, Font, VirtualKeyCode};

pub struct TownLabelArtist<T> {
    cx: T,
    params: TownArtistParameters,
    font: Arc<Font>,
    state: TownLabelArtistState,
    binding: Button,
}

impl<T> TownLabelArtist<T>
where
    T: GetNationDescription
        + GetSettlement
        + SendEngineCommands
        + Settlements
        + WithWorld
        + Send
        + Sync,
{
    pub fn new(cx: T, params: TownArtistParameters) -> TownLabelArtist<T> {
        TownLabelArtist {
            cx,
            params,
            font: Arc::new(Font::from_file("resources/fonts/roboto_slab_20.fnt")),
            state: TownLabelArtistState::NameOnly,
            binding: Button::Key(VirtualKeyCode::L),
        }
    }

    pub async fn init(&self) {
        self.on_switch().await;
    }

    pub async fn update_label(&self, settlement: Settlement) {
        match self.state {
            TownLabelArtistState::NoLabels => (),
            _ => self.update_settlement(&settlement).await,
        }
    }

    async fn on_switch(&self) {
        match self.state {
            TownLabelArtistState::NoLabels => self.erase_all().await,
            _ => self.draw_all().await,
        }
    }

    async fn erase_all(&self) {
        for settlement in self.cx.settlements().await {
            self.erase_settlement(&settlement).await;
        }
    }

    async fn erase_settlement(&self, settlement: &Settlement) {
        let command = Command::Erase(get_name(settlement));
        self.cx.send_engine_commands(vec![command]).await;
    }

    async fn draw_all(&self) {
        for settlement in self.cx.settlements().await {
            self.draw_settlement(&settlement).await;
        }
    }

    async fn draw_settlement(&self, settlement: &Settlement) {
        if settlement.class != SettlementClass::Town {
            return;
        }
        let name = get_name(&settlement);
        let text = self.state.get_label(settlement);
        let world_coord = self.get_world_coord(settlement).await;
        let draw_order = -settlement.current_population as i32;
        let commands = draw_label(name, &text, world_coord, &self.font, draw_order);
        self.cx.send_engine_commands(commands).await;
    }

    async fn get_world_coord(&self, settlement: &Settlement) -> WorldCoord {
        let mut world_coord = self
            .cx
            .with_world(|world| get_house_base_coord(world, &settlement.position, &self.params))
            .await;
        world_coord.z +=
            get_house_height_with_roof(&self.params, settlement) + self.params.label_float;
        world_coord
    }

    async fn update_settlement(&self, settlement: &Settlement) {
        if self.cx.get_settlement(&settlement.position).await.is_some() {
            self.draw_settlement(settlement).await;
        } else {
            self.erase_settlement(settlement).await;
        }
    }

    async fn change_state(&mut self) {
        self.state = self.state.next();
        self.on_switch().await;
    }
}

fn get_name(settlement: &Settlement) -> String {
    format!("settlement-label-{:?}", settlement.position)
}

fn get_house_base_coord(
    world: &World,
    house_position: &V2<usize>,
    params: &TownArtistParameters,
) -> WorldCoord {
    WorldCoord::new(
        house_position.x as f32 + 0.5,
        house_position.y as f32 + 0.5,
        get_base_z(world, house_position, params.house_width),
    )
}

fn get_base_z(world: &World, house_position: &V2<usize>, house_width: f32) -> f32 {
    let [a, b, c, d] = get_house_base_corners(world, house_position, house_width);
    let zs = [a.z, b.z, c.z, d.z];
    *zs.iter().max_by(unsafe_ordering).unwrap()
}

#[async_trait]
impl<T> HandleEngineEvent for TownLabelArtist<T>
where
    T: GetNationDescription
        + GetSettlement
        + SendEngineCommands
        + Settlements
        + WithWorld
        + Send
        + Sync
        + 'static,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        match *event {
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers,
                ..
            } if *button == self.binding && modifiers.alt() && !modifiers.ctrl() => {
                self.change_state().await
            }
            _ => (),
        }
        Capture::No
    }
}

enum TownLabelArtistState {
    NoLabels,
    NameOnly,
    NameAndPopulation,
}

impl TownLabelArtistState {
    fn get_label(&self, settlement: &Settlement) -> String {
        match self {
            TownLabelArtistState::NoLabels => String::new(),
            TownLabelArtistState::NameOnly => settlement.name.to_string(),
            TownLabelArtistState::NameAndPopulation => format!(
                "{} ({})",
                settlement.name,
                settlement.current_population.round() as usize
            ),
        }
    }

    fn next(&self) -> TownLabelArtistState {
        match self {
            TownLabelArtistState::NoLabels => TownLabelArtistState::NameOnly,
            TownLabelArtistState::NameOnly => TownLabelArtistState::NameAndPopulation,
            TownLabelArtistState::NameAndPopulation => TownLabelArtistState::NoLabels,
        }
    }
}
