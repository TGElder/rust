use crate::avatar::{Avatar, AvatarLoad, AvatarState, Rotation};
use crate::game::{CaptureEvent, Game, GameEvent, GameEventConsumer, GameParams, GameState};
use crate::game_event_consumers::VisibilityHandlerMessage;
use crate::homeland_start::{random_homeland_start, HomelandStart};
use crate::settlement::{Settlement, SettlementClass};
use crate::world::World;
use commons::rand::prelude::*;
use commons::update::UpdateSender;
use commons::V2;
use isometric::{Color, Event};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::sync::Arc;

const AVATAR_NAME: &str = "avatar";
const HANDLE: &str = "setup_homelands";

pub struct SetupNewWorld {
    game_tx: UpdateSender<Game>,
    visibility_tx: Sender<VisibilityHandlerMessage>,
}

impl SetupNewWorld {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        visibility_tx: &Sender<VisibilityHandlerMessage>,
    ) -> SetupNewWorld {
        SetupNewWorld {
            game_tx: game_tx.clone_with_handle(HANDLE),
            visibility_tx: visibility_tx.clone(),
        }
    }

    fn new_game(&self, game_state: &GameState) {
        let params = &game_state.params;
        let seed = params.seed;
        let mut rng: SmallRng = SeedableRng::seed_from_u64(seed);
        let world = &game_state.world;
        let homeland_starts = gen_homeland_starts(world, &mut rng, params.homelands);
        let avatars = gen_avatars(&homeland_starts);
        let settlements = gen_settlements(params, &homeland_starts);
        self.game_tx
            .update(move |game| setup_game(game, avatars, settlements));

        let visited = get_visited_positions(&homeland_starts);
        self.visibility_tx
            .send(VisibilityHandlerMessage { visited })
            .unwrap();
    }
}

fn gen_homeland_starts<R: Rng>(world: &World, rng: &mut R, amount: usize) -> Vec<HomelandStart> {
    (0..amount)
        .map(|_| random_homeland_start(world, rng))
        .collect()
}

fn get_visited_positions(homeland_starts: &[HomelandStart]) -> HashSet<V2<usize>> {
    homeland_starts
        .iter()
        .flat_map(|start| &start.voyage)
        .cloned()
        .collect()
}

fn gen_avatars(homeland_starts: &[HomelandStart]) -> HashMap<String, Avatar> {
    let mut avatars = HashMap::new();
    avatars.insert(
        AVATAR_NAME.to_string(),
        Avatar {
            name: AVATAR_NAME.to_string(),
            state: AvatarState::Stationary {
                position: homeland_starts[0].pre_landfall,
                rotation: Rotation::Up,
            },
            load: AvatarLoad::None,
        },
    );
    avatars
}

fn homeland_colors() -> [Color; 8] {
    [
        Color::new(1.0, 0.0, 0.0, 1.0),
        Color::new(1.0, 0.0, 1.0, 1.0),
        Color::new(0.0, 1.0, 0.0, 1.0),
        Color::new(0.0, 0.0, 1.0, 1.0),
        Color::new(1.0, 0.5, 0.0, 1.0),
        Color::new(0.5, 1.0, 0.0, 1.0),
        Color::new(1.0, 1.0, 1.0, 1.0),
        Color::new(0.0, 0.0, 0.0, 1.0),
    ]
}

fn gen_settlements(
    params: &GameParams,
    homeland_starts: &[HomelandStart],
) -> HashMap<V2<usize>, Settlement> {
    let colors = homeland_colors();
    homeland_starts
        .iter()
        .enumerate()
        .map(|(i, start)| {
            get_settlement(
                params,
                start,
                &colors
                    .get(i)
                    .expect("Not enough colors for all homeland starts"),
            )
        })
        .map(|settlement| (settlement.position, settlement))
        .collect()
}

fn get_settlement(
    params: &GameParams,
    homeland_start: &HomelandStart,
    color: &Color,
) -> Settlement {
    Settlement {
        class: SettlementClass::Homeland,
        position: homeland_start.homeland,
        name: format!(
            "homeland {},{}",
            homeland_start.homeland.x, homeland_start.homeland.y
        ),
        color: *color,
        current_population: 0.0,
        target_population: 0.0,
        gap_half_life: Some(params.homeland_distance),
    }
}

fn setup_game(
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
