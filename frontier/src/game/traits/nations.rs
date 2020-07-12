use crate::game::Game;
use crate::nation::Nation;
use std::collections::HashMap;

pub trait Nations {
    fn nations(&self) -> &HashMap<String, Nation>;
    fn mut_nations(&mut self) -> &mut HashMap<String, Nation>;

    fn get_nation(&self, name: &str) -> Option<&Nation> {
        self.nations().get(name)
    }

    fn get_nation_unsafe(&mut self, name: &str) -> &Nation {
        self.nations()
            .get(name)
            .unwrap_or_else(|| panic!("Unknown nation {}!", name))
    }

    fn mut_nation(&mut self, name: &str) -> Option<&mut Nation> {
        self.mut_nations().get_mut(name)
    }

    fn mut_nation_unsafe(&mut self, name: &str) -> &mut Nation {
        self.mut_nations()
            .get_mut(name)
            .unwrap_or_else(|| panic!("Unknown nation {}!", name))
    }
}

impl Nations for HashMap<String, Nation> {
    fn nations(&self) -> &HashMap<String, Nation> {
        &self
    }

    fn mut_nations(&mut self) -> &mut HashMap<String, Nation> {
        self
    }
}

impl Nations for Game {
    fn nations(&self) -> &HashMap<String, Nation> {
        &self.game_state.nations
    }

    fn mut_nations(&mut self) -> &mut HashMap<String, Nation> {
        &mut self.game_state.nations
    }
}
