use crate::settlement::{Settlement, SettlementClass};
use crate::system::HandleEngineEvent;
use crate::traits::{
    AddTown, GetSettlement, Micros, NationDescriptions, RandomTownName, RemoveTown, SetWorldObject,
};
use commons::async_trait::async_trait;
use commons::V2;
use isometric::coords::WorldCoord;
use isometric::{Button, ElementState, Event, VirtualKeyCode};
use std::sync::Arc;
use std::time::Duration;

pub struct TownBuilderActor<T> {
    tx: T,
    binding: Button,
    world_coord: Option<WorldCoord>,
}

impl<T> TownBuilderActor<T>
where
    T: AddTown
        + GetSettlement
        + Micros
        + NationDescriptions
        + RandomTownName
        + RemoveTown
        + SetWorldObject,
{
    pub fn new(tx: T) -> TownBuilderActor<T> {
        TownBuilderActor {
            tx,
            binding: Button::Key(VirtualKeyCode::H),
            world_coord: None,
        }
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    async fn toggle_town(&mut self) {
        let position = unwrap_or!(self.get_position(), return);
        if self.tx.get_settlement(position).await.is_some() {
            self.remove_town(position).await;
        } else {
            self.add_town(position).await;
        }
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|world_coord| world_coord.to_v2_floor())
    }

    async fn add_town(&mut self, position: V2<usize>) {
        let nation = self.random_nation().await;
        let name = self.tx.random_town_name(nation.clone()).await.unwrap();
        let last_population_update_micros = self.tx.micros().await;

        let town = Settlement {
            position,
            class: SettlementClass::Town,
            name,
            nation,
            current_population: 10.0,
            target_population: 0.0,
            gap_half_life: Duration::from_secs(0),
            last_population_update_micros,
        };

        self.tx.add_town(town).await;
    }

    async fn random_nation(&self) -> String {
        self.tx
            .nation_descriptions()
            .await
            .into_iter()
            .next()
            .unwrap()
            .name
    }

    async fn remove_town(&mut self, position: V2<usize>) {
        self.tx.remove_town(position).await;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for TownBuilderActor<T>
where
    T: AddTown
        + GetSettlement
        + Micros
        + NationDescriptions
        + RandomTownName
        + RemoveTown
        + SetWorldObject
        + Send
        + Sync
        + 'static,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) {
        match *event {
            Event::WorldPositionChanged(world_coord) => self.update_world_coord(world_coord),
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers,
                ..
            } if *button == self.binding && !modifiers.alt() && modifiers.ctrl() => {
                self.toggle_town().await
            }
            _ => (),
        }
    }
}
