use crate::game::Game;
use crate::route::Route;
use std::collections::HashMap;

pub trait Routes {
    fn routes(&self) -> &HashMap<String, Route>;
    fn routes_mut(&mut self) -> &mut HashMap<String, Route>;
}

impl Routes for HashMap<String, Route> {
    fn routes(&self) -> &HashMap<String, Route> {
        self
    }

    fn routes_mut(&mut self) -> &mut HashMap<String, Route> {
        self
    }
}

impl Routes for Game {
    fn routes(&self) -> &HashMap<String, Route> {
        &self.game_state.routes
    }

    fn routes_mut(&mut self) -> &mut HashMap<String, Route> {
        &mut self.game_state.routes
    }
}
