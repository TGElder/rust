use super::*;

use crate::settlement::*;
use isometric::drawing::{draw_house, DrawHouseParams};

const HANDLE: &str = "town_houses";

pub struct TownHouses {
    game_tx: UpdateSender<Game>,
}

impl TownHouses {
    pub fn new(game_tx: &UpdateSender<Game>) -> TownHouses {
        TownHouses {
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    fn update_settlement(&mut self, game_state: &GameState, settlement: &Settlement) {
        if game_state.settlements.contains_key(&settlement.position) {
            self.draw_settlement(game_state, settlement)
        } else {
            self.erase_settlement(settlement)
        }
    }

    fn draw_settlement(&mut self, game_state: &GameState, settlement: &Settlement) {
        if let Settlement {
            class: SettlementClass::Town,
            position,
            color,
            ..
        } = settlement
        {
            let params = game_state.params.town_artist;
            let draw_house_params = DrawHouseParams {
                width: params.house_width,
                height: get_house_height_without_roof(&params, settlement),
                roof_height: params.house_roof_height,
                base_color: *color,
                light_direction: game_state.params.light_direction,
            };
            let commands = draw_house(
                get_name(settlement),
                &game_state.world,
                &settlement.position,
                &draw_house_params,
            );
            let position = *position;
            self.game_tx.update(move |game| {
                game.force_object(WorldObject::None, position);
                game.send_engine_commands(commands);
            });
        }
    }

    fn erase_settlement(&mut self, settlement: &Settlement) {
        let command = Command::Erase(get_name(settlement));
        self.game_tx
            .update(move |game| game.send_engine_commands(vec![command]));
    }

    fn draw_all(&mut self, game_state: &GameState) {
        for settlement in game_state.settlements.values() {
            self.draw_settlement(&game_state, &settlement);
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.draw_all(game_state);
    }
}

fn get_name(settlement: &Settlement) -> String {
    format!("house-{:?}", settlement.position)
}

impl GameEventConsumer for TownHouses {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(&game_state),
            GameEvent::SettlementUpdated(settlement) => {
                self.update_settlement(game_state, settlement)
            }
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
