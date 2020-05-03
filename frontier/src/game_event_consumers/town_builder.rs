use super::*;
use crate::settlement::*;
use isometric::coords::*;
use isometric::Color;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "town_builder";

pub struct TownBuilder {
    house_color: Color,
    game_tx: UpdateSender<Game>,
    binding: Button,
    world_coord: Option<WorldCoord>,
}

impl TownBuilder {
    pub fn new(house_color: Color, game_tx: &UpdateSender<Game>) -> TownBuilder {
        TownBuilder {
            house_color,
            game_tx: game_tx.clone_with_handle(HANDLE),
            binding: Button::Key(VirtualKeyCode::H),
            world_coord: None,
        }
    }

    fn update_world_coord(&mut self, world_coord: WorldCoord) {
        self.world_coord = Some(world_coord);
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|world_coord| world_coord.to_v2_floor())
    }

    fn toggle_town(&mut self, game_state: &GameState) {
        let position = match self.get_position() {
            Some(position) => position,
            None => return,
        };
        if game_state.settlements.contains_key(&position) {
            self.remove_town(position);
        } else {
            self.add_town(position);
        }
    }

    fn add_town(&mut self, position: V2<usize>) {
        let settlement = Settlement {
            position,
            color: self.house_color,
            class: SettlementClass::Town,
            current_population: 0.0,
            target_population: 0.0,
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
