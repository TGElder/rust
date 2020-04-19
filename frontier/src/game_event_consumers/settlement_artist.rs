use super::*;

use crate::artists::HouseArtist;
use crate::settlement::*;

const HANDLE: &str = "settlement_artist";

pub struct SettlementArtist {
    command_tx: Sender<Vec<Command>>,
    state: Option<SettlementArtistState>,
}

struct SettlementArtistState {
    house_artist: HouseArtist,
}

impl SettlementArtist {
    pub fn new(command_tx: Sender<Vec<Command>>) -> SettlementArtist {
        SettlementArtist {
            command_tx,
            state: None,
        }
    }

    fn update_settlement(&mut self, game_state: &GameState, settlement: &Settlement) {
        if game_state.settlements.contains_key(&settlement.position) {
            self.draw_settlement(game_state, settlement)
        } else {
            self.erase_settlement(game_state, settlement)
        }
    }

    fn draw_settlement(&mut self, game_state: &GameState, settlement: &Settlement) {
        let state = match &self.state {
            Some(state) => state,
            None => return,
        };
        if let Settlement {
            class: SettlementClass::Town,
            position,
            color,
            population,
        } = settlement
        {
            let commands = state.house_artist.draw_house_at(
                &game_state.world,
                position,
                *color,
                house_height(*population),
            );
            self.command_tx.send(commands).unwrap();
        }
    }

    fn erase_settlement(&mut self, game_state: &GameState, settlement: &Settlement) {
        let state = match &self.state {
            Some(state) => state,
            None => return,
        };
        if let Settlement {
            class: SettlementClass::Town,
            position,
            ..
        } = settlement
        {
            let commands = state
                .house_artist
                .erase_house_at(&game_state.world, position);
            self.command_tx.send(commands).unwrap();
        }
    }

    fn draw_all(&mut self, game_state: &GameState) {
        for settlement in game_state.settlements.values() {
            self.draw_settlement(&game_state, &settlement);
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.state = Some(SettlementArtistState {
            house_artist: HouseArtist::new(game_state.params.light_direction),
        });
        self.draw_all(game_state);
    }
}

fn house_height(population: usize) -> f32 {
    0.5 + (population as f32 / 100.0)
}

impl GameEventConsumer for SettlementArtist {
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
