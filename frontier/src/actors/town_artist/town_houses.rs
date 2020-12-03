use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::game::GameEvent;
use crate::settlement::*;
use crate::traits::{GetNationDescription, GetSettlement, SendParameters, SendWorld, Settlements};
use crate::world::World;
use commons::async_channel::{Receiver, RecvError};
use commons::fn_sender::{FnMessageExt, FnReceiver};
use commons::futures::future::FutureExt;
use commons::log::info;
use commons::V2;
use isometric::drawing::{draw_house, DrawHouseParams};
use isometric::{Command, Event};

pub struct TownHouses<X> {
    x: X,
    rx: FnReceiver<TownHouses<X>>,
    engine_rx: Receiver<Arc<Event>>,
    game_rx: Receiver<GameEvent>,
    command_tx: Sender<Vec<Command>>,
    run: bool,
}

impl<X> TownHouses<X>
where
    X: GetNationDescription + GetSettlement + SendWorld + SendParameters + Settlements + Send,
{
    pub fn new(
        x: X,
        rx: FnReceiver<TownHouses<X>>,
        engine_rx: Receiver<Arc<Event>>,
        game_rx: Receiver<GameEvent>,
        command_tx: Sender<Vec<Command>>,
    ) -> TownHouses<X> {
        TownHouses {
            x,
            rx,
            engine_rx,
            game_rx,
            command_tx,
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

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        match *event.unwrap() {
            Event::Shutdown => self.shutdown(),
            _ => (),
        }
    }

    async fn handle_game_event(&mut self, event: Result<GameEvent, RecvError>) {
        if let GameEvent::Init = event.unwrap() {
            self.init().await;
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }

    async fn init(&mut self) {
        self.draw_all().await;
    }

    pub async fn update_settlement(&mut self, settlement: Settlement) {
        if self.x.get_settlement(settlement.position).await.is_some() {
            self.draw_settlement(settlement).await
        } else {
            self.erase_settlement(settlement)
        }
    }

    async fn draw_settlement(&mut self, settlement: Settlement) {
        if let Settlement {
            class: SettlementClass::Town,
            position,
            nation,
            ..
        } = settlement
        {
            let (params, light_direction) = self
                .x
                .send_parameters(|params| (params.town_artist, params.light_direction))
                .await;
            let nation = self
                .x
                .get_nation_description(nation.clone())
                .await
                .unwrap_or_else(|| panic!("Unknown nation {}", &nation));
            let draw_house_params = DrawHouseParams {
                width: params.house_width,
                height: 1.0,
                roof_height: params.house_roof_height,
                base_color: nation.color,
                light_direction,
            };

            let commands = self
                .x
                .send_world(move |world| draw_house_at_position(world, position, draw_house_params))
                .await;
            self.command_tx.send(commands).unwrap();
        }
    }

    fn erase_settlement(&mut self, settlement: Settlement) {
        let command = Command::Erase(get_name(&settlement.position));
        self.command_tx.send(vec![command]).unwrap();
    }

    async fn draw_all(&mut self) {
        for settlement in self.x.settlements().await {
            self.draw_settlement(settlement).await;
        }
    }
}

pub fn draw_house_at_position(
    world: &World,
    position: V2<usize>,
    params: DrawHouseParams,
) -> Vec<Command> {
    draw_house(get_name(&position), world, &position, &params)
}

fn get_name(position: &V2<usize>) -> String {
    format!("house-{:?}", position)
}
