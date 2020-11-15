use super::*;

use crate::settlement::*;
use isometric::drawing::{draw_house, DrawHouseParams};

const NAME: &str = "town_houses";

pub struct TownHouses {
    game_tx: FnSender<Game>,
}

impl TownHouses {
    pub fn new(game_tx: &FnSender<Game>) -> TownHouses {
        TownHouses {
            game_tx: game_tx.clone_with_name(NAME),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.draw_all(game_state);
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
            nation,
            ..
        } = settlement
        {
            let params = game_state.params.town_artist;
            let nation = game_state
                .nations
                .get(nation)
                .unwrap_or_else(|| panic!("Unknown nation {}", &nation));
            let draw_house_params = DrawHouseParams {
                width: params.house_width,
                height: get_house_height_without_roof(&params, settlement),
                roof_height: params.house_roof_height,
                base_color: *nation.color(),
                light_direction: game_state.params.light_direction,
            };
            let commands = draw_house(
                get_name(settlement),
                &game_state.world,
                &position,
                &draw_house_params,
            );
            self.game_tx.send(move |game| {
                game.send_engine_commands(commands);
            });
        }
    }

    fn erase_settlement(&mut self, settlement: &Settlement) {
        let command = Command::Erase(get_name(settlement));
        self.game_tx
            .send(move |game| game.send_engine_commands(vec![command]));
    }

    fn draw_all(&mut self, game_state: &GameState) {
        for settlement in game_state.settlements.values() {
            self.draw_settlement(&game_state, &settlement);
        }
    }
}

fn get_name(settlement: &Settlement) -> String {
    format!("house-{:?}", settlement.position)
}

impl GameEventConsumer for TownHouses {
    fn name(&self) -> &'static str {
        NAME
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
