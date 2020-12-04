use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::actors::town_artist::get_house_height_without_roof;
use crate::actors::TownArtistParameters;
use crate::game::GameEvent;
use crate::settlement::*;
use crate::traits::{GetNationDescription, GetSettlement, SendWorld, Settlements};
use crate::world::World;
use commons::async_channel::{Receiver, RecvError};
use commons::fn_sender::{FnMessageExt, FnReceiver};
use commons::futures::future::FutureExt;
use commons::V2;
use isometric::drawing::{draw_house, DrawHouseParams};
use isometric::{Color, Command, Event};

pub struct TownHouseArtist<X> {
    x: X,
    rx: FnReceiver<TownHouseArtist<X>>,
    engine_rx: Receiver<Arc<Event>>,
    game_rx: Receiver<GameEvent>,
    command_tx: Sender<Vec<Command>>,
    params: TownArtistParameters,
    run: bool,
}

impl<X> TownHouseArtist<X>
where
    X: GetNationDescription + GetSettlement + SendWorld + Settlements + Send,
{
    pub fn new(
        x: X,
        rx: FnReceiver<TownHouseArtist<X>>,
        engine_rx: Receiver<Arc<Event>>,
        game_rx: Receiver<GameEvent>,
        command_tx: Sender<Vec<Command>>,
        params: TownArtistParameters,
    ) -> TownHouseArtist<X> {
        TownHouseArtist {
            x,
            rx,
            engine_rx,
            game_rx,
            command_tx,
            params,
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

    pub async fn update_settlement(&mut self, settlement: Settlement) {
        if self.x.get_settlement(settlement.position).await.is_some() {
            self.draw_settlement(settlement).await
        } else {
            self.erase_settlement(settlement)
        }
    }

    async fn draw_settlement(&mut self, settlement: Settlement) {
        if settlement.class != SettlementClass::Town {
            return;
        }
        let params = DrawHouseParams {
            width: self.params.house_width,
            height: get_house_height_without_roof(&self.params, &settlement),
            roof_height: self.params.house_roof_height,
            base_color: self.get_nation_color(settlement.nation.clone()).await,
            light_direction: self.params.light_direction,
        };

        self.draw_house(params, settlement).await;
    }

    async fn get_nation_color(&mut self, nation: String) -> Color {
        self.x
            .get_nation_description(nation)
            .await
            .unwrap_or_else(|| panic!("Unknown nation"))
            .color
    }

    async fn draw_house(&mut self, params: DrawHouseParams, settlement: Settlement) {
        let name = get_name(&settlement.position);
        let position = settlement.position;
        let commands = self
            .x
            .send_world(move |world| get_draw_commands(name, world, position, params))
            .await;
        self.command_tx.send(commands).unwrap();
    }

    fn erase_settlement(&mut self, settlement: Settlement) {
        let command = Command::Erase(get_name(&settlement.position));
        self.command_tx.send(vec![command]).unwrap();
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        if let Event::Shutdown = *event.unwrap() {
            self.shutdown()
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }

    async fn handle_game_event(&mut self, event: Result<GameEvent, RecvError>) {
        if let GameEvent::Init = event.unwrap() {
            self.init().await;
        }
    }

    async fn init(&mut self) {
        self.draw_all().await;
    }

    async fn draw_all(&mut self) {
        for settlement in self.x.settlements().await {
            self.draw_settlement(settlement).await;
        }
    }
}

pub fn get_draw_commands(
    name: String,
    world: &World,
    position: V2<usize>,
    params: DrawHouseParams,
) -> Vec<Command> {
    draw_house(name, world, &position, &params)
}

fn get_name(position: &V2<usize>) -> String {
    format!("house-{:?}", position)
}
