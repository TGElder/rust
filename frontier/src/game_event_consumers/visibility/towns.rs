use crate::game::*;
use crate::settlement::SettlementClass;
use commons::grid::Grid;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

use crate::actors::Visibility;
use commons::fn_sender::FnSender;

const HANDLE: &str = "visibility_from_towns";

pub struct VisibilityFromTowns<D>
where
    D: FnSender<Visibility>,
{
    tx: D,
}

impl<D> VisibilityFromTowns<D>
where
    D: FnSender<Visibility> + Clone,
{
    pub fn new(tx: &D) -> VisibilityFromTowns<D> {
        VisibilityFromTowns { tx: tx.clone() }
    }

    fn tick(&mut self, game_state: &GameState) {
        let visited = town_visited_cells(game_state).collect();
        self.tx
            .send(|visibility| visibility.check_visibility_and_reveal(visited));
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

impl<D> GameEventConsumer for VisibilityFromTowns<D>
where
    D: FnSender<Visibility> + Clone + Send,
{
    fn name(&self) -> &'static str {
        HANDLE
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
