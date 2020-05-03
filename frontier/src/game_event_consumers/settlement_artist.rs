use super::*;

use crate::artists::HouseArtist;
use crate::settlement::*;

const HANDLE: &str = "settlement_artist";

pub struct SettlementArtist {
    game_tx: UpdateSender<Game>,
    state: Option<SettlementArtistState>,
}

struct SettlementArtistState {
    house_artist: HouseArtist,
}

impl SettlementArtist {
    pub fn new(game_tx: &UpdateSender<Game>) -> SettlementArtist {
        SettlementArtist {
            game_tx: game_tx.clone_with_handle(HANDLE),
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
            ..
        } = settlement
        {
            let house_height = house_height(settlement);
            let roof_height = roof_height();
            let commands = state.house_artist.draw_house_at(
                &game_state.world,
                position,
                *color,
                house_height,
                roof_height,
            );
            self.add_void_then_send_commands(
                house_height + roof_height,
                settlement.position,
                commands,
            );
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
            self.add_void_then_send_commands(0.0, settlement.position, commands);
        }
    }

    fn add_void_then_send_commands(
        &mut self,
        void_height: f32,
        position: V2<usize>,
        commands: Vec<Command>,
    ) {
        self.game_tx.update(move |game| {
            game.force_object(
                WorldObject::Void {
                    height: void_height,
                },
                position,
            );
            game.send_engine_commands(commands);
        });
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

fn roof_height() -> f32 {
    0.5
}

fn house_height(settlement: &Settlement) -> f32 {
    (settlement.current_population + 1.0).log(10.0) as f32
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
