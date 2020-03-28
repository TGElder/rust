use super::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "follow_avatar";

pub struct FollowAvatar {
    command_tx: Sender<Vec<Command>>,
    game_tx: UpdateSender<Game>,
    binding: Button,
}

impl FollowAvatar {
    pub fn new(command_tx: Sender<Vec<Command>>, game_tx: &UpdateSender<Game>) -> FollowAvatar {
        FollowAvatar {
            command_tx,
            game_tx: game_tx.clone_with_handle(HANDLE),
            binding: Button::Key(VirtualKeyCode::C),
        }
    }

    fn look_at_selected_avatar(&self, game_state: &GameState) {
        if game_state.follow_avatar {
            if let Some(Avatar { state, .. }) = &game_state.selected_avatar() {
                let maybe_world_coord =
                    state.compute_world_coord(&game_state.world, &game_state.game_micros);
                self.command_tx
                    .send(vec![Command::LookAt(maybe_world_coord)])
                    .unwrap();
                return;
            }
        }
        self.command_tx.send(vec![Command::LookAt(None)]).unwrap();
    }

    fn toggle_follow_avatar(&mut self) {
        self.game_tx.update(toggle_follow_avatar);
    }
}

fn toggle_follow_avatar(game: &mut Game) {
    let game_state = game.mut_state();
    game_state.follow_avatar = !game_state.follow_avatar;
}

impl GameEventConsumer for FollowAvatar {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
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
                self.toggle_follow_avatar();
            }
        }
        if let Event::Tick = *event {
            self.look_at_selected_avatar(game_state);
        }

        CaptureEvent::No
    }

    fn shutdown(&mut self) {}

    fn is_shutdown(&self) -> bool {
        true
    }
}