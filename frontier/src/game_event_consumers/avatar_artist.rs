use super::*;
use crate::artists::{AvatarArtist, AvatarArtistParams};

const NAME: &str = "avatar_artist_handler";

pub struct AvatarArtistHandler {
    command_tx: Sender<Vec<Command>>,
    avatar_artist: Option<AvatarArtist>,
}

impl AvatarArtistHandler {
    pub fn new(command_tx: Sender<Vec<Command>>) -> AvatarArtistHandler {
        AvatarArtistHandler {
            command_tx,
            avatar_artist: None,
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.init_avatar_artist(game_state);
    }

    fn init_avatar_artist(&mut self, game_state: &GameState) {
        let avatar_artist =
            AvatarArtist::new(AvatarArtistParams::new(&game_state.params.light_direction));
        self.avatar_artist = Some(avatar_artist);
    }

    fn draw_avatars(&mut self, game_state: &GameState) {
        if let Some(avatar_artist) = &mut self.avatar_artist {
            let commands =
                avatar_artist.update_avatars(&game_state.avatars, &game_state.game_micros);
            self.command_tx.send(commands).unwrap();
        }
    }
}

impl GameEventConsumer for AvatarArtistHandler {
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Init = event {
            self.init(game_state);
        };
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Tick = *event {
            self.draw_avatars(game_state);
        }
        CaptureEvent::No
    }
}
