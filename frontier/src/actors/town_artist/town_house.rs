use crate::actors::TownArtistParameters;
use crate::settlement::*;
use crate::traits::{
    GetNationDescription, GetSettlement, SendEngineCommands, Settlements, WithWorld,
};
use commons::V2;
use isometric::drawing::{create_and_update_house_drawing, House};
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
        let house = House {
            position: &settlement.position,
            width: &self.params.house_width,
            height: &self.params.house_height,
            roof_height: &self.params.house_roof_height,
            base_color: &self.get_nation_color(&settlement.nation).await,
            light_direction: &self.params.light_direction,
            rotated: false,
        };

        self.draw_house(house).await;
    }

    async fn get_nation_color(&self, nation: &str) -> Color {
        self.cx
            .get_nation_description(nation)
            .await
            .unwrap_or_else(|| panic!("Unknown nation"))
            .colors
            .primary
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    async fn draw_house<'a>(&self, house: House<'a>) {
        let name = get_name(house.position);
        let commands = self
            .cx
            .with_world(|world| create_and_update_house_drawing(name, world, vec![house]))
            .await;
        self.cx.send_engine_commands(commands).await;
    }

    async fn erase_settlement(&self, settlement: Settlement) {
        let command = Command::Erase(get_name(&settlement.position));
        self.cx.send_engine_commands(vec![command]).await;
    }
}

fn get_name(position: &V2<usize>) -> String {
    format!("town-house-{:?}", position)
}
