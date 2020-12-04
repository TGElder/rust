use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::actors::town_artist::get_house_height_with_roof;
use crate::actors::TownArtistParameters;
use crate::game::GameEvent;
use crate::settlement::*;
use crate::traits::{GetNationDescription, GetSettlement, SendWorld, Settlements};
use crate::world::World;
use commons::async_channel::{Receiver, RecvError};
use commons::fn_sender::{FnMessageExt, FnReceiver};
use commons::futures::future::FutureExt;
use commons::{unsafe_ordering, V2};
use isometric::coords::WorldCoord;
use isometric::drawing::{draw_label, get_house_base_corners};
use isometric::{Button, Command, ElementState, Event, Font, ModifiersState, VirtualKeyCode};

const LABEL_FLOAT: f32 = 0.33;

pub struct TownLabelArtist<X> {
    x: X,
    rx: FnReceiver<TownLabelArtist<X>>,
    engine_rx: Receiver<Arc<Event>>,
    game_rx: Receiver<GameEvent>,
    command_tx: Sender<Vec<Command>>,
    params: TownArtistParameters,
    font: Arc<Font>,
    state: TownLabelArtistState,
    binding: Button,
    run: bool,
}

impl<X> TownLabelArtist<X>
where
    X: GetNationDescription + GetSettlement + SendWorld + Settlements + Send,
{
    pub fn new(
        x: X,
        rx: FnReceiver<TownLabelArtist<X>>,
        engine_rx: Receiver<Arc<Event>>,
        game_rx: Receiver<GameEvent>,
        command_tx: Sender<Vec<Command>>,
        params: TownArtistParameters,
    ) -> TownLabelArtist<X> {
        TownLabelArtist {
            x,
            rx,
            engine_rx,
            game_rx,
            command_tx,
            params,
            font: Arc::new(Font::from_file("resources/fonts/roboto_slab_20.fnt")),
            state: TownLabelArtistState::NameOnly,
            binding: Button::Key(VirtualKeyCode::L),
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                mut message = self.rx.get_message().fuse() => message.apply(self).await,
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await,
                event = self.game_rx.recv().fuse() => self.handle_game_event(event).await
            }
        }
    }

    pub async fn update_label(&mut self, settlement: Settlement) {
        match self.state {
            TownLabelArtistState::NoLabels => (),
            _ => self.update_settlement(&settlement).await,
        }
    }

    async fn update_settlement(&mut self, settlement: &Settlement) {
        if self.x.get_settlement(settlement.position).await.is_some() {
            self.draw_settlement(settlement).await;
        } else {
            self.erase_settlement(settlement);
        }
    }

    async fn draw_settlement(&mut self, settlement: &Settlement) {
        if settlement.class != SettlementClass::Town {
            return;
        }
        let name = get_name(&settlement);
        let text = self.state.get_label(settlement);
        let params = self.params;
        let position = settlement.position;
        let mut world_coord = self
            .x
            .send_world(move |world| get_house_base_coord(world, position, params))
            .await;
        world_coord.z += get_house_height_with_roof(&params, settlement) + LABEL_FLOAT;
        let draw_order = -settlement.current_population as i32;

        let commands = draw_label(name, &text, world_coord, &self.font, draw_order);

        self.command_tx.send(commands).unwrap();
    }

    fn erase_settlement(&mut self, settlement: &Settlement) {
        let command = Command::Erase(get_name(settlement));
        self.command_tx.send(vec![command]).unwrap();
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        match *event.unwrap() {
            Event::Shutdown => self.shutdown(),
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers: ModifiersState { alt: true, .. },
                ..
            } if *button == self.binding => self.change_state().await,
            _ => (),
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }

    async fn change_state(&mut self) {
        self.state = self.state.next();
        self.on_switch().await;
    }

    async fn on_switch(&mut self) {
        match self.state {
            TownLabelArtistState::NoLabels => self.erase_all().await,
            _ => self.draw_all().await,
        }
    }

    async fn draw_all(&mut self) {
        for settlement in self.x.settlements().await {
            self.draw_settlement(&settlement).await;
        }
    }

    async fn erase_all(&mut self) {
        for settlement in self.x.settlements().await {
            self.erase_settlement(&settlement);
        }
    }

    async fn handle_game_event(&mut self, event: Result<GameEvent, RecvError>) {
        if let GameEvent::Init = event.unwrap() {
            self.init().await;
        }
    }

    async fn init(&mut self) {
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
    let z = get_base_z(world, &house_position, params.house_width);
    WorldCoord::new(
        house_position.x as f32 + 0.5,
        house_position.y as f32 + 0.5,
        z,
    )
}

fn get_base_z(world: &World, house_position: &V2<usize>, house_width: f32) -> f32 {
    let [a, b, c, d] = get_house_base_corners(world, house_position, house_width);
    let zs = [a.z, b.z, c.z, d.z];
    *zs.iter().max_by(unsafe_ordering).unwrap()
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
