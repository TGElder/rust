use std::sync::Arc;

use commons::async_trait::async_trait;
use isometric::{Button, Command, ElementState, Event, VirtualKeyCode};

use crate::artists::{AvatarArtist, AvatarDrawCommand, FastAvatarArtist};
use crate::avatars::Avatars;
use crate::system::{Capture, HandleEngineEvent};
use crate::traits::{Micros, SendEngineCommands, SendRotate, WithAvatars};

pub struct AvatarArtistActor<T> {
    cx: T,
    avatar_artist: Option<AvatarArtist>,
    fast_avatar_artist: FastAvatarArtist,
    follow_avatar: bool,
    follow_avatar_binding: Button,
}

impl<T> AvatarArtistActor<T>
where
    T: Micros + SendEngineCommands + SendRotate + WithAvatars + Send + Sync,
{
    pub fn new(
        cx: T,
        avatar_artist: AvatarArtist,
        fast_avatar_artist: FastAvatarArtist,
    ) -> AvatarArtistActor<T> {
        AvatarArtistActor {
            cx,
            avatar_artist: Some(avatar_artist),
            fast_avatar_artist,
            follow_avatar: true,
            follow_avatar_binding: Button::Key(VirtualKeyCode::C),
        }
    }

    pub async fn init(&self) {
        self.setup_engine_for_follow_avatar_setting().await;
        self.cx
            .send_engine_commands(self.fast_avatar_artist.init())
            .await
    }

    async fn setup_engine_for_follow_avatar_setting(&self) {
        if !self.follow_avatar {
            self.cx
                .send_engine_commands(vec![Command::LookAt(None)])
                .await;
        }

        let rotate_over_undrawn = self.follow_avatar;
        self.cx.send_rotate_background(move |rotate| {
            rotate.set_rotate_over_undrawn(rotate_over_undrawn)
        });
    }

    async fn draw_avatars(&mut self) {
        let mut avatar_artist = self.avatar_artist.take().unwrap();
        let micros = self.cx.micros().await;

        let (commands, avatar_artist) = self
            .cx
            .with_avatars(|avatars| {
                let draw_commands = get_draw_commands(avatars);
                let mut engine_commands = avatar_artist.update_avatars(&draw_commands, &micros);

                let mut fast_commands = self
                    .fast_avatar_artist
                    .draw_avatars(&mut avatars.all.values(), &micros);
                engine_commands.append(&mut fast_commands);

                if self.follow_avatar {
                    engine_commands.push(look_at_selected(avatars, &micros));
                }

                (engine_commands, avatar_artist)
            })
            .await;

        self.cx.send_engine_commands(commands).await;
        self.avatar_artist = Some(avatar_artist);
    }

    async fn toggle_follow_avatar(&mut self) {
        self.follow_avatar = !self.follow_avatar;
        self.setup_engine_for_follow_avatar_setting().await;
    }
}

fn get_draw_commands(avatars: &Avatars) -> Vec<AvatarDrawCommand> {
    avatars
        .all
        .values()
        .map(|avatar| AvatarDrawCommand {
            avatar,
            draw_when_done: Some(&avatar.name) == avatars.selected.as_ref(),
        })
        .collect()
}

fn look_at_selected(avatars: &Avatars, micros: &u128) -> Command {
    Command::LookAt(
        avatars
            .selected()
            .and_then(|avatar| avatar.journey.as_ref())
            .map(|journey| journey.world_coord_at(micros)),
    )
}

#[async_trait]
impl<T> HandleEngineEvent for AvatarArtistActor<T>
where
    T: Micros + SendEngineCommands + SendRotate + WithAvatars + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        match *event {
            Event::Tick => self.draw_avatars().await,
            Event::Button {
                ref button,
                state: ElementState::Pressed,
                modifiers,
                ..
            } if button == &self.follow_avatar_binding && !modifiers.alt() => {
                self.toggle_follow_avatar().await
            }
            _ => (),
        }
        Capture::No
    }
}
