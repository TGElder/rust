use super::*;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

pub struct FollowAvatar {
    command_tx: Sender<GameCommand>,
    binding: Button,
}

impl FollowAvatar {
    pub fn new(command_tx: Sender<GameCommand>) -> FollowAvatar {
        FollowAvatar {
            command_tx,
            binding: Button::Key(VirtualKeyCode::C),
        }
    }

    fn follow(&mut self, game_state: &GameState) {
        if game_state.follow_avatar {
            if let Some(world_coord) = game_state
                .avatar_state
                .compute_world_coord(&game_state.world, &game_state.game_micros)
            {
                self.command_tx
                    .send(GameCommand::EngineCommands(vec![Command::LookAt(
                        world_coord,
                    )]))
                    .unwrap();
            }
        }
    }
}

impl GameEventConsumer for FollowAvatar {
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
                self.command_tx
                    .send(GameCommand::FollowAvatar(!game_state.follow_avatar))
                    .unwrap();
            }
        }
        if let Event::DrawingWorld = *event {
            self.follow(&game_state);
        }

        CaptureEvent::No
    }
}
