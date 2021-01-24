use std::sync::mpsc::Sender;

use crate::actors::town_artist::get_house_height_without_roof;
use crate::actors::TownArtistParameters;
use crate::settlement::*;
use crate::traits::{GetNationDescription, GetSettlement, SendWorld, Settlements};
use crate::world::World;
use commons::V2;
use isometric::drawing::{draw_house, DrawHouseParams};
use isometric::{Color, Command};

pub struct TownHouseArtist<T> {
    tx: T,
    command_tx: Sender<Vec<Command>>,
    params: TownArtistParameters,
}

impl<T> TownHouseArtist<T>
where
    T: GetNationDescription + GetSettlement + SendWorld + Settlements + Send,
{
    pub fn new(
        tx: T,
        command_tx: Sender<Vec<Command>>,
        params: TownArtistParameters,
    ) -> TownHouseArtist<T> {
        TownHouseArtist {
            tx,
            command_tx,
            params,
        }
    }

    pub async fn init(&mut self) {
        self.draw_all().await;
    }

    pub async fn update_settlement(&mut self, settlement: Settlement) {
        if self.tx.get_settlement(settlement.position).await.is_some() {
            self.draw_settlement(settlement).await
        } else {
            self.erase_settlement(settlement)
        }
    }

    async fn draw_all(&mut self) {
        for settlement in self.tx.settlements().await {
            self.draw_settlement(settlement).await;
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
        self.tx
            .get_nation_description(nation)
            .await
            .unwrap_or_else(|| panic!("Unknown nation"))
            .colors
            .primary
    }

    async fn draw_house(&mut self, params: DrawHouseParams, settlement: Settlement) {
        let name = get_name(&settlement.position);
        let position = settlement.position;
        let commands = self
            .tx
            .send_world(move |world| get_draw_commands(name, world, position, params))
            .await;
        self.command_tx.send(commands).unwrap();
    }

    fn erase_settlement(&mut self, settlement: Settlement) {
        let command = Command::Erase(get_name(&settlement.position));
        self.command_tx.send(vec![command]).unwrap();
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
