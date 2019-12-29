use super::*;
use crate::avatar::*;
use crate::pathfinder::*;
use commons::v2;

pub const NAME: &str = "farm_candidates";

pub struct FarmCandidateHandler {
    pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
}

impl FarmCandidateHandler {
    pub fn new(
        pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    ) -> FarmCandidateHandler {
        FarmCandidateHandler { pathfinder_tx }
    }

    fn init(&mut self, game_state: &GameState) {
        let function: Box<dyn FnOnce(&mut Pathfinder<AvatarTravelDuration>) + Send> =
            Box::new(move |pathfinder| {
                pathfinder.init_targets(NAME.to_string());
            });
        self.pathfinder_tx
            .send(PathfinderCommand::Update(function))
            .unwrap();
        self.update_all(game_state);
    }

    fn update_all(&mut self, game_state: &GameState) {
        let mut positions = vec![];
        for x in 0..game_state.world.width() {
            for y in 0..game_state.world.height() {
                positions.push(v2(x, y));
            }
        }
        self.update_positions(game_state, positions);
    }

    fn update_positions(&mut self, game_state: &GameState, positions: Vec<V2<usize>>) {
        let positions: Vec<(V2<usize>, bool)> = positions
            .into_iter()
            .map(|position| (position, game_state.is_farm_candidate(&position)))
            .collect();
        let function: Box<dyn FnOnce(&mut Pathfinder<AvatarTravelDuration>) + Send> =
            Box::new(move |pathfinder| {
                positions
                    .iter()
                    .for_each(|(position, target)| pathfinder.load_target(NAME, position, *target));
            });
        self.pathfinder_tx
            .send(PathfinderCommand::Update(function))
            .unwrap();
    }
}

impl GameEventConsumer for FarmCandidateHandler {
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
}
