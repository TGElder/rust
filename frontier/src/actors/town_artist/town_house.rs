use crate::actors::TownArtistParameters;
use crate::settlement::*;
use crate::traits::{
    GetNationDescription, GetSettlement, SendEngineCommands, Settlements, WithWorld,
};
use crate::world::World;
use commons::V2;
use isometric::drawing::{draw_house, DrawHouseParams};
use isometric::{Color, Command};

pub struct TownHouseArtist<T> {
    cx: T,
    params: TownArtistParameters,
}

impl<T> TownHouseArtist<T>
where
    T: GetNationDescription + GetSettlement + SendEngineCommands + Settlements + WithWorld + Send,
{
    pub fn new(cx: T, params: TownArtistParameters) -> TownHouseArtist<T> {
        TownHouseArtist { cx, params }
    }

    pub async fn init(&self) {
        self.draw_all().await;
    }

    pub async fn update_settlement(&self, settlement: Settlement) {
        if self.cx.get_settlement(&settlement.position).await.is_some() {
            self.draw_settlement(settlement).await
        } else {
            self.erase_settlement(settlement).await
        }
    }

    async fn draw_all(&self) {
        for settlement in self.cx.settlements().await {
            self.draw_settlement(settlement).await;
        }
    }

    async fn draw_settlement(&self, settlement: Settlement) {
        if settlement.class != SettlementClass::Town {
            return;
        }
        let params = DrawHouseParams {
            width: self.params.house_width,
            height: self.params.house_height,
            roof_height: self.params.house_roof_height,
            base_color: self.get_nation_color(&settlement.nation).await,
            light_direction: self.params.light_direction,
        };

        self.draw_house(params, settlement).await;
    }

    async fn get_nation_color(&self, nation: &str) -> Color {
        self.cx
            .get_nation_description(&nation)
            .await
            .unwrap_or_else(|| panic!("Unknown nation"))
            .colors
            .primary
    }

    async fn draw_house(&self, params: DrawHouseParams, settlement: Settlement) {
        let name = get_name(&settlement.position);
        let commands = self
            .cx
            .with_world(|world| get_draw_commands(name, world, &settlement.position, params))
            .await;
        self.cx.send_engine_commands(commands).await;
    }

    async fn erase_settlement(&self, settlement: Settlement) {
        let command = Command::Erase(get_name(&settlement.position));
        self.cx.send_engine_commands(vec![command]).await;
    }
}

pub fn get_draw_commands(
    name: String,
    world: &World,
    position: &V2<usize>,
    params: DrawHouseParams,
) -> Vec<Command> {
    draw_house(name, world, position, &params)
}

fn get_name(position: &V2<usize>) -> String {
    format!("house-{:?}", position)
}
