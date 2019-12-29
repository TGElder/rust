use super::*;
use crate::avatar::*;
use crate::pathfinder::*;

pub const NAME: &str = "town_candidates";

pub struct TownCandidateHandler {
    pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
}

impl TownCandidateHandler {
    pub fn new(
        pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    ) -> TownCandidateHandler {
        TownCandidateHandler { pathfinder_tx }
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
        for town in game_state.territory.controllers() {
            self.update_town(game_state, &town);
        }
    }

    fn update_town(&mut self, game_state: &GameState, town: &V2<usize>) {
        let target = self.town_has_plots(game_state, town);
        let town = *town;
        let function: Box<dyn FnOnce(&mut Pathfinder<AvatarTravelDuration>) + Send> =
            Box::new(move |pathfinder| {
                pathfinder.load_target(NAME, &town, target);
            });
        self.pathfinder_tx
            .send(PathfinderCommand::Update(function))
            .unwrap();
    }

    fn town_has_plots(&self, game_state: &GameState, town: &V2<usize>) -> bool {
        game_state
            .territory
            .controlled_territory(town)
            .iter()
            .any(|position| game_state.is_farm_candidate(position))
    }

    fn territory_changed(&mut self, game_state: &GameState, changes: &[TerritoryChange]) {
        let towns: HashSet<&V2<usize>> = changes.iter().map(|change| &change.controller).collect();
        for town in towns {
            self.update_town(game_state, town);
        }
    }

    fn farm_built_or_destroyed(&mut self, game_state: &GameState, farm: &V2<usize>) {
        if let Some(town) = game_state
            .territory
            .who_controls_tile(&game_state.world, farm)
        {
            self.update_town(game_state, &town)
        }
    }
}

impl GameEventConsumer for TownCandidateHandler {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::TerritoryChanged(changes) => self.territory_changed(game_state, changes),
            GameEvent::ObjectUpdated {
                object: WorldObject::Farm,
                position,
                ..
            } => self.farm_built_or_destroyed(game_state, position),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
