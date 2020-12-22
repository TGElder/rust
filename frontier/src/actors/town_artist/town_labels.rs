use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::actors::town_artist::get_house_height_with_roof;
use crate::actors::TownArtistParameters;
use crate::configuration::HandleEngineEvent;
use crate::settlement::*;
use crate::traits::{GetNationDescription, GetSettlement, SendWorld, Settlements};
use crate::world::World;
use commons::async_trait::async_trait;
use commons::{unsafe_ordering, V2};
use isometric::coords::WorldCoord;
use isometric::drawing::{draw_label, get_house_base_corners};
use isometric::{Button, Command, ElementState, Event, Font, ModifiersState, VirtualKeyCode};

pub struct TownLabelArtist<X> {
    x: X,
    command_tx: Sender<Vec<Command>>,
    params: TownArtistParameters,
    font: Arc<Font>,
    state: TownLabelArtistState,
    binding: Button,
}

impl<X> TownLabelArtist<X>
where
    X: GetNationDescription + GetSettlement + SendWorld + Settlements + Send,
{
    pub fn new(
        x: X,
        command_tx: Sender<Vec<Command>>,
        params: TownArtistParameters,
    ) -> TownLabelArtist<X> {
        TownLabelArtist {
            x,
            command_tx,
            params,
            font: Arc::new(Font::from_file("resources/fonts/roboto_slab_20.fnt")),
            state: TownLabelArtistState::NameOnly,
            binding: Button::Key(VirtualKeyCode::L),
        }
    }

    pub async fn init(&mut self) {
        self.on_switch().await;
    }

    pub async fn update_label(&mut self, settlement: Settlement) {
        match self.state {
            TownLabelArtistState::NoLabels => (),
            _ => self.update_settlement(&settlement).await,
        }
    }

    async fn on_switch(&mut self) {
        match self.state {
            TownLabelArtistState::NoLabels => self.erase_all().await,
            _ => self.draw_all().await,
        }
    }

    async fn erase_all(&mut self) {
        for settlement in self.x.settlements().await {
            self.erase_settlement(&settlement);
        }
    }

    fn erase_settlement(&mut self, settlement: &Settlement) {
        let command = Command::Erase(get_name(settlement));
        self.command_tx.send(vec![command]).unwrap();
    }

    async fn draw_all(&mut self) {
        for settlement in self.x.settlements().await {
            self.draw_settlement(&settlement).await;
        }
    }

    async fn draw_settlement(&mut self, settlement: &Settlement) {
        if settlement.class != SettlementClass::Town {
            return;
        }
        let name = get_name(&settlement);
        let text = self.state.get_label(settlement);
        let world_coord = self.get_world_coord(settlement).await;
        let draw_order = -settlement.current_population as i32;
        let commands = draw_label(name, &text, world_coord, &self.font, draw_order);
        self.command_tx.send(commands).unwrap();
    }

    async fn get_world_coord(&mut self, settlement: &Settlement) -> WorldCoord {
        let params = self.params;
        let position = settlement.position;
        let mut world_coord = self
            .x
            .send_world(move |world| get_house_base_coord(world, position, params))
            .await;
        world_coord.z += get_house_height_with_roof(&params, settlement) + params.label_float;
        world_coord
    }

    async fn update_settlement(&mut self, settlement: &Settlement) {
        if self.x.get_settlement(settlement.position).await.is_some() {
            self.draw_settlement(settlement).await;
        } else {
            self.erase_settlement(settlement);
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
    house_position: V2<usize>,
    params: TownArtistParameters,
) -> WorldCoord {
    WorldCoord::new(
        house_position.x as f32 + 0.5,
        house_position.y as f32 + 0.5,
        get_base_z(world, &house_position, params.house_width),
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
    T: GetNationDescription + GetSettlement + SendWorld + Settlements + Send + Sync + 'static,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) {
        match *event {
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers,
                ..
            } if *button == self.binding
                && (modifiers == (ModifiersState::ALT & !ModifiersState::CTRL)) =>
            {
                self.change_state().await
            }
            _ => (),
        }
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
