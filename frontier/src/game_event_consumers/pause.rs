use super::*;
use crate::simulation::Simulation;
use commons::futures::executor::block_on;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "pause";

pub struct Pause {
    game_tx: UpdateSender<Game>,
    sim_tx: UpdateSender<Simulation>,
    binding: Button,
    pause: bool,
}

impl Pause {
    pub fn new(game_tx: &UpdateSender<Game>, sim_tx: &UpdateSender<Simulation>) -> Pause {
        Pause {
            game_tx: game_tx.clone_with_handle(HANDLE),
            sim_tx: sim_tx.clone_with_handle(HANDLE),
            binding: Button::Key(VirtualKeyCode::Space),
            pause: true,
        }
    }

    fn pause(&mut self) {
        let game_tx = self.game_tx.clone();
        self.sim_tx.update(move |sim| {
            block_on(async {
                sim.pause_persistent();
                game_tx.update(|game| game.mut_state().speed = 0.0).await;
            })
        });
    }

    fn resume(&mut self, default_speed: f32) {
        let game_tx = self.game_tx.clone();
        self.sim_tx.update(move |sim| {
            block_on(async {
                sim.resume_persistent();
                game_tx
                    .update(move |game| game.mut_state().speed = default_speed)
                    .await;
            })
        });
    }
}

impl GameEventConsumer for Pause {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Init = event {
            if game_state.speed == 0.0 {
                self.pause = false;
            }
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                if self.pause {
                    self.pause();
                } else {
                    self.resume(game_state.params.default_speed);
                }
                self.pause = !self.pause;
            }
        }
        CaptureEvent::No
    }
}
