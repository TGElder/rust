use crate::game::Game;
use crate::route::{RouteSet, RouteSetKey};
use std::collections::HashMap;

pub trait Routes {
    fn routes(&self) -> &HashMap<RouteSetKey, RouteSet>;
    fn routes_mut(&mut self) -> &mut HashMap<RouteSetKey, RouteSet>;
}

impl Routes for HashMap<RouteSetKey, RouteSet> {
    fn routes(&self) -> &HashMap<RouteSetKey, RouteSet> {
        self
    }

    fn routes_mut(&mut self) -> &mut HashMap<RouteSetKey, RouteSet> {
        self
    }
}

impl Routes for Game {
    fn routes(&self) -> &HashMap<RouteSetKey, RouteSet> {
        &self.game_state.routes
    }

    fn routes_mut(&mut self) -> &mut HashMap<RouteSetKey, RouteSet> {
        &mut self.game_state.routes
    }
}
