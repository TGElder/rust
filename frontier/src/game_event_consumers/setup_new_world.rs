use crate::actors::VisibilityHandlerMessage;
use crate::avatar::{Avatar, AvatarLoad, AvatarState, Rotation};
use crate::game::{
    CaptureEvent, Game, GameEvent, GameEventConsumer, GameParams, GameState, HomelandParams,
};
use crate::homeland_start::{HomelandEdge, HomelandStart, HomelandStartGen};
use crate::nation::{skin_colors, Nation};
use crate::settlement::{Settlement, SettlementClass};
use crate::world::World;
use commons::async_channel::Sender;
use commons::futures::executor::block_on;
use commons::grid::Grid;
use commons::rand::prelude::*;
use commons::update::UpdateSender;
use commons::V2;
use isometric::{Color, Event};
use std::collections::{HashMap, HashSet};
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
        let homeland_starts = gen_homeland_starts(world, &mut rng, &params.homeland);
        let avatars = gen_avatars(&mut rng, &homeland_starts, params.avatar_color);
        let nations = gen_nations(&mut rng, &params);
        let settlements = gen_settlements(params, &homeland_starts, &nations);
        self.game_tx
            .update(move |game| setup_game(game, avatars, nations, settlements));

        let visited = get_visited_positions(&homeland_starts);
        block_on(
            self.visibility_tx
                .send(VisibilityHandlerMessage { visited }),
        )
        .unwrap();
    }
}

fn gen_homeland_starts<R: Rng>(
    world: &World,
    rng: &mut R,
    params: &HomelandParams,
) -> Vec<HomelandStart> {
    let min_distance_between_homelands =
        min_distance_between_homelands(world, params.count, &params.edges);
    let mut gen = HomelandStartGen::new(
        world,
        rng,
        &params.edges,
        Some(min_distance_between_homelands),
    );
    (0..params.count).map(|_| gen.random_start()).collect()
}

fn min_distance_between_homelands(
    world: &World,
    homelands: usize,
    edges: &[HomelandEdge],
) -> usize {
    (total_edge_positions(world, edges) as f32 / (homelands as f32 * 2.0)).ceil() as usize
}

fn total_edge_positions(world: &World, edges: &[HomelandEdge]) -> usize {
    edges
        .iter()
        .map(|edge| match edge {
            HomelandEdge::North => world.width(),
            HomelandEdge::East => world.height(),
            HomelandEdge::South => world.width(),
            HomelandEdge::West => world.height(),
        })
        .sum()
}

fn get_visited_positions(homeland_starts: &[HomelandStart]) -> HashSet<V2<usize>> {
    homeland_starts[0].voyage.iter().cloned().collect()
}

fn gen_avatars<R: Rng>(
    rng: &mut R,
    homeland_starts: &[HomelandStart],
    color: Color,
) -> HashMap<String, Avatar> {
    let mut avatars = HashMap::new();
    avatars.insert(
        AVATAR_NAME.to_string(),
        Avatar {
            name: AVATAR_NAME.to_string(),
            state: AvatarState::Stationary {
                position: homeland_starts[0].pre_landfall,
                rotation: Rotation::Up,
            },
            color,
            skin_color: avatar_skin_color(rng),
            load: AvatarLoad::None,
        },
    );
    avatars
}

fn avatar_skin_color<R: Rng>(rng: &mut R) -> Color {
    *skin_colors().choose(rng).unwrap()
}

fn gen_nations<R: Rng>(rng: &mut R, params: &GameParams) -> HashMap<String, Nation> {
    params
        .nations
        .choose_multiple(rng, params.homeland.count)
        .map(|nation| (nation.name.clone(), Nation::from_description(nation)))
        .collect()
}

fn gen_settlements(
    params: &GameParams,
    homeland_starts: &[HomelandStart],
    nations: &HashMap<String, Nation>,
) -> HashMap<V2<usize>, Settlement> {
    nations
        .keys()
        .enumerate()
        .map(|(i, nation)| get_settlement(params, &homeland_starts[i], nation.to_string()))
        .map(|settlement| (settlement.position, settlement))
        .collect()
}

fn get_settlement(
    params: &GameParams,
    homeland_start: &HomelandStart,
    nation: String,
) -> Settlement {
    Settlement {
        class: SettlementClass::Homeland,
        position: homeland_start.homeland,
        name: nation.clone(),
        nation,
        current_population: 0.0,
        target_population: 0.0,
        gap_half_life: params.homeland_distance.mul_f32(5.19), // 5.19 makes half life equivalent to '7/8th life'
        last_population_update_micros: 0,
    }
}

fn setup_game(
    game: &mut Game,
    avatars: HashMap<String, Avatar>,
    nations: HashMap<String, Nation>,
    settlements: HashMap<V2<usize>, Settlement>,
) {
    let game_state = game.mut_state();
    game_state.avatars = avatars;
    game_state.nations = nations;
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

#[cfg(test)]
mod tests {
    use super::*;
    use commons::M;

    #[test]
    fn test_min_distance_between_homelands() {
        let world = World::new(M::zeros(1024, 512), 0.5);
        let edges = vec![HomelandEdge::East, HomelandEdge::West];
        assert_eq!(min_distance_between_homelands(&world, 8, &edges), 64);
    }

    #[test]
    fn test_min_distance_between_homelands_rounds_up() {
        let world = World::new(M::zeros(1024, 512), 0.5);
        let edges = vec![HomelandEdge::East, HomelandEdge::West];
        assert_eq!(min_distance_between_homelands(&world, 9, &edges), 57);
    }
}
