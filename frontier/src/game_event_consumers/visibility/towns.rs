use crate::game::*;
use crate::settlement::SettlementClass;
use crate::traits::Visibility;
use commons::grid::Grid;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

const NAME: &str = "visibility_from_towns";

pub struct VisibilityFromTowns<T>
where
    T: Visibility,
{
    visibility: T,
}

impl<T> VisibilityFromTowns<T>
where
    T: Visibility,
{
    pub fn new(visibility: T) -> VisibilityFromTowns<T> {
        VisibilityFromTowns { visibility }
    }

    fn tick(&mut self, game_state: &GameState) {
        let visited = town_visited_cells(game_state).collect();
        self.visibility.check_visibility_and_reveal(visited);
    }
}

fn town_visited_cells<'a>(game_state: &'a GameState) -> impl Iterator<Item = V2<usize>> + 'a {
    let world = &game_state.world;
    game_state
        .settlements
        .iter()
        .filter(|(_, settlement)| settlement.class == SettlementClass::Town)
        .flat_map(move |(position, _)| world.get_corners_in_bounds(position))
}

impl<T> GameEventConsumer for VisibilityFromTowns<T>
where
    T: Visibility + Send,
{
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Tick { .. } = event {
            self.tick(game_state);
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
