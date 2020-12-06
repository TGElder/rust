use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{
    AddTown, GetSettlement, Micros, NationDescriptions, RandomTownName, RemoveTown, SetWorldObject,
};
use commons::async_channel::{Receiver, RecvError};
use commons::futures::future::FutureExt;
use commons::V2;
use isometric::coords::WorldCoord;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};
use std::sync::Arc;
use std::time::Duration;

pub struct TownBuilderActor<T> {
    x: T,
    engine_rx: Receiver<Arc<Event>>,
    binding: Button,
    world_coord: Option<WorldCoord>,
    run: bool,
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
    pub fn new(x: T, engine_rx: Receiver<Arc<Event>>) -> TownBuilderActor<T> {
        TownBuilderActor {
            x,
            engine_rx,
            binding: Button::Key(VirtualKeyCode::H),
            world_coord: None,
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        let event: Arc<Event> = event.unwrap();

        match *event {
            Event::WorldPositionChanged(world_coord) => self.update_world_coord(world_coord),
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers: ModifiersState { alt: false, .. },
                ..
            } if *button == self.binding => self.toggle_town().await,
            Event::Shutdown => self.shutdown(),
            _ => (),
        }
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    async fn toggle_town(&mut self) {
        let position = unwrap_or!(self.get_position(), return);
        if self.x.get_settlement(position).await.is_some() {
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
        let nation = self
            .x
            .nation_descriptions()
            .await
            .into_iter()
            .next()
            .unwrap()
            .name;
        let name = self.x.random_town_name(nation.clone()).await.unwrap();
        let last_population_update_micros = self.x.micros().await;

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

        self.x.add_town(town).await;
    }

    async fn remove_town(&mut self, position: V2<usize>) {
        self.x.remove_town(position).await;
    }

    fn shutdown(&mut self) {
        self.run = false;
    }
}
