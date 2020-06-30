use crate::game::Game;
use crate::route::{Route, RouteKey};
use std::collections::HashMap;

pub trait Routes {
    fn routes(&self) -> &HashMap<RouteKey, Route>;
    fn routes_mut(&mut self) -> &mut HashMap<RouteKey, Route>;
}

impl Routes for HashMap<RouteKey, Route> {
    fn routes(&self) -> &HashMap<RouteKey, Route> {
        self
    }

    fn routes_mut(&mut self) -> &mut HashMap<RouteKey, Route> {
        self
    }
}

impl Routes for Game {
    fn routes(&self) -> &HashMap<RouteKey, Route> {
        &self.game_state.routes
    }

    fn routes_mut(&mut self) -> &mut HashMap<RouteKey, Route> {
        &mut self.game_state.routes
    }
}
