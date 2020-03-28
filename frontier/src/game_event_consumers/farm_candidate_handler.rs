use super::*;
use crate::pathfinder::*;
use commons::v2;
use isometric::cell_traits::WithVisibility;

const HANDLE: &str = "farm_candidate_handler";
pub const FARM_CANDIDATE_TARGETS: &str = "farm_candidates";

pub struct FarmCandidateHandler {
    pathfinder_tx: UpdateSender<Pathfinder<AvatarTravelDuration>>,
}

impl FarmCandidateHandler {
    pub fn new(
        pathfinder_tx: &UpdateSender<Pathfinder<AvatarTravelDuration>>,
    ) -> FarmCandidateHandler {
        FarmCandidateHandler {
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        let candidate_map = candidate_map_all_positions(game_state);
        self.pathfinder_tx.update(move |pathfinder| {
            pathfinder.init_targets(FARM_CANDIDATE_TARGETS.to_string());
            update_pathfinder(pathfinder, candidate_map);
        });
    }

    fn update_all(&mut self, game_state: &GameState) {
        let candidate_map = candidate_map_all_positions(game_state);
        self.pathfinder_tx.update(move |pathfinder| {
            update_pathfinder(pathfinder, candidate_map);
        });
    }

    fn update_positions(&mut self, game_state: &GameState, positions: Vec<V2<usize>>) {
        let candidate_map = candidate_map(game_state, positions);
        self.pathfinder_tx.update(move |pathfinder| {
            update_pathfinder(pathfinder, candidate_map);
        });
    }
}

fn candidate_map_all_positions(game_state: &GameState) -> Vec<(V2<usize>, bool)> {
    let mut positions = vec![];
    for x in 0..game_state.world.width() {
        for y in 0..game_state.world.height() {
            positions.push(v2(x, y));
        }
    }
    candidate_map(game_state, positions)
}

fn candidate_map(game_state: &GameState, positions: Vec<V2<usize>>) -> Vec<(V2<usize>, bool)> {
    positions
        .into_iter()
        .map(|position| (position, is_farm_candidate(game_state, &position)))
        .collect()
}

pub fn is_farm_candidate(game_state: &GameState, position: &V2<usize>) -> bool {
    let constraints = &game_state.params.farm_constraints;
    let beach_level = game_state.params.world_gen.beach_level;
    let world = &game_state.world;
    if position.x == world.width() - 1 || position.y == world.height() - 1 {
        return false;
    };
    match world.tile_avg_temperature(&position) {
        Some(temperature) if temperature >= constraints.min_temperature => (),
        _ => return false,
    };
    match world.tile_avg_groundwater(&position) {
        Some(groundwater) if groundwater >= constraints.min_groundwater => (),
        _ => return false,
    };
    world
        .get_cell(position)
        .map(|cell| {
            cell.is_visible()
                && cell.object == WorldObject::None
                && world.get_max_abs_rise(position) <= constraints.max_slope
                && world.get_lowest_corner(position) > beach_level
        })
        .unwrap_or(false)
}

fn update_pathfinder(
    pathfinder: &mut Pathfinder<AvatarTravelDuration>,
    candidate_map: Vec<(V2<usize>, bool)>,
) {
    candidate_map.iter().for_each(|(position, target)| {
        pathfinder.load_target(FARM_CANDIDATE_TARGETS, position, *target)
    });
}

impl GameEventConsumer for FarmCandidateHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::CellsRevealed(selection) => {
                match selection {
                    CellSelection::All => self.update_all(game_state),
                    CellSelection::Some(positions) => {
                        self.update_positions(game_state, positions.to_vec())
                    }
                };
            }
            GameEvent::ObjectUpdated { position, .. } => {
                self.update_positions(game_state, vec![*position])
            }
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }

    fn shutdown(&mut self) {}

    fn is_shutdown(&self) -> bool {
        true
    }
}
