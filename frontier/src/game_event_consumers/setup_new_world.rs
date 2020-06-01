use crate::avatar::{Avatar, AvatarLoad, AvatarState};
use crate::game::{CaptureEvent, Game, GameEvent, GameEventConsumer, GameParams, GameState};
use crate::settlement::{Settlement, SettlementClass};
use crate::shore_start::shore_start;
use crate::shore_start::ShoreStart;
use commons::rand::prelude::*;
use commons::update::UpdateSender;
use commons::V2;
use isometric::Event;
use std::collections::HashMap;
use std::sync::Arc;

const AVATAR_NAME: &str = "avatar";
const HOMELAND_NAME: &str = "homeland";
const HANDLE: &str = "setup_homelands";

pub struct SetupNewWorld {
    game_tx: UpdateSender<Game>,
}

impl SetupNewWorld {
    pub fn new(game_tx: &UpdateSender<Game>) -> SetupNewWorld {
        SetupNewWorld {
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    fn new_game(&self, game_state: &GameState) {
        let params = &game_state.params;
        let seed = params.seed;
        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);
        let world = &game_state.world;
        let shore_start = shore_start(32, &world, &mut rng);
        let avatars = gen_avatars(&shore_start);
        let settlements = get_settlements(params, &shore_start);
        self.game_tx
            .update(move |game| update_game(game, avatars, settlements));
    }
}

fn gen_avatars(shore_start: &ShoreStart) -> HashMap<String, Avatar> {
    let mut avatars = HashMap::new();
    avatars.insert(
        AVATAR_NAME.to_string(),
        Avatar {
            name: AVATAR_NAME.to_string(),
            state: AvatarState::Stationary {
                position: shore_start.origin(),
                rotation: shore_start.rotation(),
            },
            load: AvatarLoad::None,
        },
    );
    avatars
}

fn get_settlements(
    params: &GameParams,
    shore_start: &ShoreStart,
) -> HashMap<V2<usize>, Settlement> {
    let mut settlements = HashMap::new();
    settlements.insert(
        shore_start.origin(),
        Settlement {
            class: SettlementClass::Homeland,
            position: shore_start.origin(),
            name: HOMELAND_NAME.to_string(),
            color: params.house_color,
            current_population: 0.0,
            target_population: 0.0,
            gap_half_life: Some(params.homeland_distance),
        },
    );
    settlements
}

fn update_game(
    game: &mut Game,
    avatars: HashMap<String, Avatar>,
    settlements: HashMap<V2<usize>, Settlement>,
) {
    let game_state = game.mut_state();
    game_state.avatars = avatars;
    game_state.settlements = settlements;
    game_state.selected_avatar = Some(AVATAR_NAME.to_string());
}

impl GameEventConsumer for SetupNewWorld {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::NewGame = event {
            self.new_game(game_state)
        };
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
