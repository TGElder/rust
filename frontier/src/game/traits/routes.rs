use crate::game::Game;
use crate::route::RouteSet;
use commons::V2;
use std::collections::HashMap;

pub trait Routes {
    fn routes(&self) -> &HashMap<V2<usize>, RouteSet>;
    fn routes_mut(&mut self) -> &mut HashMap<V2<usize>, RouteSet>;
}

impl Routes for HashMap<V2<usize>, RouteSet> {
    fn routes(&self) -> &HashMap<V2<usize>, RouteSet> {
        self
    }

    fn routes_mut(&mut self) -> &mut HashMap<V2<usize>, RouteSet> {
        self
    }
}

impl Routes for Game {
    fn routes(&self) -> &HashMap<V2<usize>, RouteSet> {
        &self.game_state.routes
    }

    fn routes_mut(&mut self) -> &mut HashMap<V2<usize>, RouteSet> {
        &mut self.game_state.routes
    }
}
