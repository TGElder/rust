use std::sync::mpsc::Sender;
use std::sync::Arc;

use commons::async_trait::async_trait;
use isometric::{Command, Event};

use crate::artists::AvatarArtist;
use crate::system::HandleEngineEvent;
use crate::traits::{Micros, SendAvatars};

pub struct AvatarArtistActor<T> {
    tx: T,
    command_tx: Sender<Vec<Command>>,
    avatar_artist: Option<AvatarArtist>,
}

impl<T> AvatarArtistActor<T>
where
    T: SendAvatars + Micros,
{
    pub fn new(
        tx: T,
        command_tx: Sender<Vec<Command>>,
        avatar_artist: AvatarArtist,
    ) -> AvatarArtistActor<T> {
        AvatarArtistActor {
            tx,
            command_tx,
            avatar_artist: Some(avatar_artist),
        }
    }

    async fn draw_avatars(&mut self) {
        let mut avatar_artist = self.avatar_artist.take().unwrap();
        let micros = self.tx.micros().await;

        let (commands, avatar_artist) = self
            .tx
            .send_avatars(move |avatars| {
                let commands = avatar_artist.update_avatars(&avatars.all, &micros);
                (commands, avatar_artist)
            })
            .await;

        self.command_tx.send(commands).unwrap();
        self.avatar_artist = Some(avatar_artist)
    }
}

#[async_trait]
impl<T> HandleEngineEvent for AvatarArtistActor<T>
where
    T: SendAvatars + Micros + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) {
        if let Event::Tick = *event {
            self.draw_avatars().await;
        }
    }
}
