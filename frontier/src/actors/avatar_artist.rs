use std::sync::mpsc::Sender;
use std::sync::Arc;

use commons::async_trait::async_trait;
use isometric::{Button, Command, ElementState, Event, VirtualKeyCode};

use crate::artists::AvatarArtist;
use crate::avatars::Avatars;
use crate::system::HandleEngineEvent;
use crate::traits::{Micros, SendAvatars};

pub struct AvatarArtistActor<T> {
    tx: T,
    command_tx: Sender<Vec<Command>>,
    avatar_artist: Option<AvatarArtist>,
    follow_avatar: bool,
    follow_avatar_binding: Button,
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
            follow_avatar: true,
            follow_avatar_binding: Button::Key(VirtualKeyCode::C),
        }
    }

    async fn draw_avatars(&mut self) {
        let mut avatar_artist = self.avatar_artist.take().unwrap();
        let micros = self.tx.micros().await;
        let follow_avatar = self.follow_avatar;

        let (commands, avatar_artist) = self
            .tx
            .send_avatars(move |avatars| {
                let mut commands = avatar_artist.update_avatars(&avatars.all, &micros);

                if follow_avatar {
                    commands.push(look_at_selected(avatars, &micros));
                }

                (commands, avatar_artist)
            })
            .await;

        self.command_tx.send(commands).unwrap();
        self.avatar_artist = Some(avatar_artist)
    }

    fn toggle_follow_avatar(&mut self) {
        self.follow_avatar = !self.follow_avatar;
        if !self.follow_avatar {
            self.command_tx.send(vec![Command::LookAt(None)]).unwrap();
        }
    }
}

fn look_at_selected(avatars: &Avatars, micros: &u128) -> Command {
    Command::LookAt(
        avatars
            .selected()
            .and_then(|avatar| avatar.state.compute_world_coord(micros)),
    )
}

#[async_trait]
impl<T> HandleEngineEvent for AvatarArtistActor<T>
where
    T: SendAvatars + Micros + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) {
        match *event {
            Event::Tick => self.draw_avatars().await,
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers,
                ..
            } if button == &self.follow_avatar_binding && !modifiers.alt() => {
                self.toggle_follow_avatar()
            }
            _ => (),
        }
    }
}
