use std::sync::Arc;

use futures::executor::block_on;
use isometric::{Command, Event, EventConsumer};

use crate::artists::AvatarArtist;
use crate::avatars::Avatars;
use crate::traits::has::HasFollowAvatar;
use crate::traits::{Micros, SendEngineCommands, SendRotate, WithAvatars};

pub struct AvatarArtistActor<T> {
    cx: T,
    avatar_artist: AvatarArtist,
    follow_avatar: bool,
}

impl<T> AvatarArtistActor<T>
where
    T: HasFollowAvatar + Micros + SendEngineCommands + SendRotate + WithAvatars + Send + Sync,
{
    pub fn new(cx: T, avatar_artist: AvatarArtist) -> AvatarArtistActor<T> {
        AvatarArtistActor {
            cx,
            avatar_artist,
            follow_avatar: true,
        }
    }

    pub async fn init(&mut self) {
        self.set_follow_avatar(self.cx.follow_avatar().await).await;
        self.cx
            .send_engine_commands(self.avatar_artist.init())
            .await
    }

    async fn set_follow_avatar(&mut self, follow_avatar: bool) {
        self.follow_avatar = follow_avatar;
        self.setup_engine_for_follow_avatar_setting().await;
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
        self.update_follow_avatar().await;

        let micros = self.cx.micros().await;

        let commands = self
            .cx
            .with_avatars(|avatars| {
                let mut commands = self.avatar_artist.draw_avatars(
                    &mut avatars.all.values(),
                    avatars.selected.as_ref(),
                    &micros,
                );

                if self.follow_avatar {
                    commands.push(look_at_selected(avatars, &micros));
                }

                commands
            })
            .await;

        self.cx.send_engine_commands(commands).await;
    }

    async fn update_follow_avatar(&mut self) {
        let follow_avatar = self.cx.follow_avatar().await;
        if self.follow_avatar != follow_avatar {
            self.set_follow_avatar(follow_avatar).await;
        }
    }
}

fn look_at_selected(avatars: &Avatars, micros: &u128) -> Command {
    Command::LookAt(
        avatars
            .selected()
            .and_then(|avatar| avatar.journey.as_ref())
            .map(|journey| journey.world_coord_at(micros)),
    )
}

impl<T> EventConsumer for AvatarArtistActor<T>
where
    T: HasFollowAvatar + Micros + SendEngineCommands + SendRotate + WithAvatars + Send + Sync,
{
    fn consume_event(&mut self, event: Arc<Event>) {
        if let Event::Tick = *event {
            block_on(self.draw_avatars())
        }
    }
}
