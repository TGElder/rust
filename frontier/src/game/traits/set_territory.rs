use crate::game::{Game, TerritoryState};

pub trait SetTerritory {
    fn set_territory(&mut self, states: Vec<TerritoryState>);
}

impl SetTerritory for Vec<TerritoryState> {
    fn set_territory(&mut self, mut states: Vec<TerritoryState>) {
        self.append(&mut states);
    }
}

impl SetTerritory for Game {
    fn set_territory(&mut self, states: Vec<TerritoryState>) {
        self.set_territory(states);
    }
}
