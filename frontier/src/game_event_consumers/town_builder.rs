use super::*;
use crate::nation::Nation;
use crate::settlement::*;
use isometric::coords::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};
use std::time::Duration;

const NAME: &str = "town_builder";

pub struct TownBuilder {
    game_tx: FnSender<Game>,
    binding: Button,
    world_coord: Option<WorldCoord>,
}

impl TownBuilder {
    pub fn new(game_tx: &FnSender<Game>) -> TownBuilder {
        TownBuilder {
            game_tx: game_tx.clone_with_name(NAME),
            binding: Button::Key(VirtualKeyCode::H),
            world_coord: None,
        }
    }

    fn update_world_coord(&mut self, world_coord: Option<WorldCoord>) {
        self.world_coord = world_coord;
    }

    fn get_position(&self) -> Option<V2<usize>> {
        self.world_coord
            .map(|world_coord| world_coord.to_v2_floor())
    }

    fn toggle_town(&mut self, game_state: &GameState) {
        let position = unwrap_or!(self.get_position(), return);
        if game_state.settlements.contains_key(&position) {
            self.remove_town(position);
        } else {
            self.add_town(position);
        }
    }

    fn add_town(&mut self, position: V2<usize>) {
        self.game_tx
            .send(move |game| add_settlement(game, position));
    }

    fn remove_town(&mut self, position: V2<usize>) {
        // self.game_tx
        // .send(move |game| game.remove_settlement(position));
    }
}

fn add_settlement(game: &mut Game, position: V2<usize>) {
    let nation = random_nation_name(game);
    let name = get_nation(game, &nation).get_town_name();

    let settlement = Settlement {
        position,
        class: SettlementClass::Town,
        name,
        nation,
        current_population: 0.0,
        target_population: 0.0,
        gap_half_life: Duration::from_secs(0),
        last_population_update_micros: game.game_state().game_micros,
    };

    // game.add_settlement(settlement);
}

fn random_nation_name(game: &Game) -> String {
    game.game_state().nations.keys().next().unwrap().clone()
}

fn get_nation<'a>(game: &'a mut Game, name: &'a str) -> &'a mut Nation {
    game.mut_state()
        .nations
        .get_mut(name)
        .unwrap_or_else(|| panic!("Unknown nation {}", &name))
}

impl GameEventConsumer for TownBuilder {
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::WorldPositionChanged(world_coord) = *event {
            self.update_world_coord(world_coord);
        } else if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                self.toggle_town(&game_state);
            }
        }
        CaptureEvent::No
    }
}
