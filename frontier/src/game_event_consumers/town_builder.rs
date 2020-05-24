use super::*;
use crate::names::Namer;
use crate::settlement::*;
use isometric::coords::*;
use isometric::Color;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "town_builder";

pub struct TownBuilder {
    house_color: Color,
    game_tx: UpdateSender<Game>,
    namer: Box<dyn Namer + Send>,
    binding: Button,
    world_coord: Option<WorldCoord>,
}

impl TownBuilder {
    pub fn new(
        house_color: Color,
        game_tx: &UpdateSender<Game>,
        namer: Box<dyn Namer + Send>,
    ) -> TownBuilder {
        TownBuilder {
            house_color,
            game_tx: game_tx.clone_with_handle(HANDLE),
            namer,
            binding: Button::Key(VirtualKeyCode::H),
            world_coord: None,
        }
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|world_coord| world_coord.to_v2_floor())
    }

    fn toggle_town(&mut self, game_state: &GameState) {
        let position = unwrap_or!(self.get_position(), return);
        if game_state.settlements.contains_key(&position) {
            self.remove_town(position);
        } else {
            self.add_town(position);
        }
    }

    fn add_town(&mut self, position: V2<usize>) {
        let settlement = Settlement {
            position,
            class: SettlementClass::Town,
            name: self.namer.next_name(),
            color: self.house_color,
            current_population: 0.0,
            target_population: 0.0,
            gap_half_life: None,
        };
        self.game_tx
            .update(move |game| game.add_settlement(settlement));
    }

    fn remove_town(&mut self, position: V2<usize>) {
        self.game_tx
            .update(move |game| game.remove_settlement(position));
    }
}

impl GameEventConsumer for TownBuilder {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        } else if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                self.toggle_town(&game_state);
            }
        }
        CaptureEvent::No
    }
}
