use super::*;

const HANDLE: &str = "avatar_artist_handler";

pub struct AvatarArtistHandler {
    command_tx: Sender<Vec<Command>>,
    avatar_artist: Option<AvatarArtist>,
    travel_mode_fn: Option<TravelModeFn>,
}

impl AvatarArtistHandler {
    pub fn new(command_tx: Sender<Vec<Command>>) -> AvatarArtistHandler {
        AvatarArtistHandler {
            command_tx,
            avatar_artist: None,
            travel_mode_fn: None,
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.init_avatar_artist(game_state);
        self.travel_mode_fn = Some(TravelModeFn::new(
            game_state.params.avatar_travel.min_navigable_river_width,
        ));
    }

    fn init_avatar_artist(&mut self, game_state: &GameState) {
        let avatar_artist = AvatarArtist::new(&game_state.params.light_direction);
        self.avatar_artist = Some(avatar_artist);
    }

    fn draw_avatars(&mut self, game_state: &GameState) {
        if let (Some(avatar_artist), Some(travel_mode_fn)) =
            (&mut self.avatar_artist, &self.travel_mode_fn)
        {
            let commands = avatar_artist.update_avatars(
                &game_state.avatars,
                &game_state.world,
                &game_state.game_micros,
                &travel_mode_fn,
            );
            self.command_tx.send(commands).unwrap();
        }
    }
}

impl GameEventConsumer for AvatarArtistHandler {
    fn name(&self) -> &'static str {
        HANDLE
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
