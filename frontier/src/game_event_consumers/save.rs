use super::*;
use crate::simulation::Simulation;
use commons::futures::executor::block_on;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "save";

pub struct Save {
    game_tx: UpdateSender<Game>,
    sim_tx: UpdateSender<Simulation>,
    binding: Button,
    path: String,
}

impl Save {
    pub fn new(game_tx: &UpdateSender<Game>, sim_tx: &UpdateSender<Simulation>) -> Save {
        Save {
            game_tx: game_tx.clone_with_handle(HANDLE),
            sim_tx: sim_tx.clone_with_handle(HANDLE),
            binding: Button::Key(VirtualKeyCode::P),
            path: "save".to_string(),
        }
    }

    fn save(&mut self) {
        let path_for_sim = self.path.clone();
        let path_for_game = self.path.clone();
        let game_tx = self.game_tx.clone();
        println!("Will save between simulation steps");
        self.sim_tx.update(move |sim| {
            block_on(async {
                println!("Saving...");
                sim.save(&path_for_sim);
                game_tx.update(|game| game.save(path_for_game)).await;
                println!("Saved");
            })
        });
    }
}

impl GameEventConsumer for Save {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                self.save();
            }
        }
        CaptureEvent::No
    }
}
