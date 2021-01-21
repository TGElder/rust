use crate::game::*;
use crate::traits::Visibility;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

use std::iter::empty;

const NAME: &str = "visibility_from_avatar";

pub struct VisibilityFromAvatar<T>
where
    T: Visibility,
{
    visibility: T,
}

impl<T> VisibilityFromAvatar<T>
where
    T: Visibility,
{
    pub fn new(visibility: T) -> VisibilityFromAvatar<T> {
        VisibilityFromAvatar { visibility }
    }

    fn tick(&mut self, game_state: &GameState, from: &u128, to: &u128) {
        let visited = avatar_visited(game_state, from, to).collect();
        self.visibility.check_visibility_and_reveal(visited);
    }
}

fn avatar_visited<'a>(
    game_state: &'a GameState,
    from: &'a u128,
    to: &'a u128,
) -> Box<dyn Iterator<Item = V2<usize>> + 'a> {
    if let Some(avatar) = game_state.avatars.selected() {
        if let Some(journey) = &avatar.journey {
            let edges = journey.edges_between_times(from, to);
            return Box::new(edges.into_iter().map(|edge| *edge.to()));
        }
    }
    Box::new(empty())
}

impl<T> GameEventConsumer for VisibilityFromAvatar<T>
where
    T: Visibility + Send,
{
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Tick {
            from_micros,
            to_micros,
        } = event
        {
            self.tick(game_state, from_micros, to_micros);
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
