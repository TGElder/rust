use isometric::coords::WorldCoord;

use crate::avatar::{Avatar, Progress};

pub struct ArtistAvatar<'a> {
    pub avatar: &'a Avatar,
    pub done: bool,
    pub progress: Progress<'a>,
    pub world_coord: WorldCoord,
}

impl<'a> ArtistAvatar<'a> {
    pub fn from(avatar: &'a Avatar, at: &u128) -> Option<ArtistAvatar<'a>> {
        let journey = avatar.journey.as_ref()?;
        let done = journey.done(at);
        let progress = journey.progress_at(at);
        let world_coord = progress.world_coord_at(at);
        Some(ArtistAvatar {
            avatar,
            done,
            progress,
            world_coord,
        })
    }
}
