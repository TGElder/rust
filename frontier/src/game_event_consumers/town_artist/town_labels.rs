use super::*;

use crate::settlement::*;
use commons::unsafe_ordering;
use isometric::coords::WorldCoord;
use isometric::drawing::{draw_text, get_house_base_corners};
use isometric::Font;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "town_labels";
const LABEL_FLOAT: f32 = 0.33;

pub struct TownLabels {
    game_tx: UpdateSender<Game>,
    font: Arc<Font>,
    state: TownLabelState,
    binding: Button,
}

enum TownLabelState {
    NoLabels,
    NameOnly,
    NameAndPopulation,
}

impl TownLabelState {
    fn get_label(&self, settlement: &Settlement) -> String {
        match self {
            TownLabelState::NoLabels => String::new(),
            TownLabelState::NameOnly => settlement.name.to_string(),
            TownLabelState::NameAndPopulation => format!(
                "{} ({})",
                settlement.name,
                settlement.current_population.round() as usize
            ),
        }
    }

    fn next(&self) -> TownLabelState {
        match self {
            TownLabelState::NoLabels => TownLabelState::NameOnly,
            TownLabelState::NameOnly => TownLabelState::NameAndPopulation,
            TownLabelState::NameAndPopulation => TownLabelState::NoLabels,
        }
    }
}

impl TownLabels {
    pub fn new(game_tx: &UpdateSender<Game>) -> TownLabels {
        TownLabels {
            font: Arc::new(Font::from_csv_and_texture("serif.csv", "serif.png")),
            game_tx: game_tx.clone_with_handle(HANDLE),
            state: TownLabelState::NameOnly,
            binding: Button::Key(VirtualKeyCode::L),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.on_switch(game_state);
    }

    fn change_state(&mut self, game_state: &GameState) {
        self.state = self.state.next();
        self.on_switch(game_state);
    }

    fn on_switch(&mut self, game_state: &GameState) {
        match self.state {
            TownLabelState::NoLabels => self.erase_all(game_state),
            _ => self.draw_all(game_state),
        }
    }

    fn on_update(&mut self, game_state: &GameState, settlement: &Settlement) {
        match self.state {
            TownLabelState::NoLabels => (),
            _ => self.update_settlement(game_state, settlement),
        }
    }

    fn update_settlement(&mut self, game_state: &GameState, settlement: &Settlement) {
        if game_state.settlements.contains_key(&settlement.position) {
            self.draw_settlement(game_state, settlement)
        } else {
            self.erase_settlement(settlement)
        }
    }

    fn draw_settlement(&mut self, game_state: &GameState, settlement: &Settlement) {
        if settlement.class != SettlementClass::Town {
            return;
        }
        let commands = draw_text(
            get_name(settlement),
            &self.state.get_label(settlement),
            get_label_position(
                &game_state.world,
                settlement,
                &game_state.params.town_artist,
            ),
            &self.font,
        );
        self.game_tx
            .update(move |game| game.send_engine_commands(commands));
    }

    fn erase_settlement(&mut self, settlement: &Settlement) {
        let command = Command::Erase(get_name(settlement));
        self.game_tx
            .update(move |game| game.send_engine_commands(vec![command]));
    }

    fn draw_all(&mut self, game_state: &GameState) {
        for settlement in game_state.settlements.values() {
            self.draw_settlement(&game_state, &settlement);
        }
    }

    fn erase_all(&mut self, game_state: &GameState) {
        for settlement in game_state.settlements.values() {
            self.erase_settlement(&settlement);
        }
    }
}

fn get_name(settlement: &Settlement) -> String {
    format!("settlement-label-{:?}", settlement.position)
}

fn get_label_position(
    world: &World,
    settlement: &Settlement,
    params: &TownArtistParameters,
) -> WorldCoord {
    let position = &settlement.position;
    let base_z = get_base_z(world, settlement, params.house_width);
    let z = base_z + get_house_height_with_roof(params, settlement) + LABEL_FLOAT;
    WorldCoord::new(position.x as f32 + 0.5, position.y as f32 + 0.5, z)
}

fn get_base_z(world: &World, settlement: &Settlement, house_width: f32) -> f32 {
    let [a, b, c, d] = get_house_base_corners(world, &settlement.position, house_width);
    let zs = [a.z, b.z, c.z, d.z];
    *zs.iter().max_by(unsafe_ordering).unwrap()
}

impl GameEventConsumer for TownLabels {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(&game_state),
            GameEvent::SettlementUpdated(settlement) => self.on_update(game_state, settlement),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: true, .. },
            ..
        } = *event
        {
            if *button == self.binding {
                self.change_state(game_state);
            }
        }
        CaptureEvent::No
    }
}
