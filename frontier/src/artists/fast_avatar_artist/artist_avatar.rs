use isometric::coords::WorldCoord;

use crate::avatar::{Avatar, Progress};

pub struct ArtistAvatar<'a> {
    pub avatar: &'a Avatar,
    pub progress: Progress<'a>,
    pub world_coord: WorldCoord,
}

impl<'a> ArtistAvatar<'a> {
    pub fn from(avatar: &'a Avatar, at: &u128) -> Option<ArtistAvatar<'a>> {
        let progress = avatar.journey.as_ref()?.progress_at(at);
        let world_coord = progress.world_coord_at(at);
        Some(ArtistAvatar {
            avatar,
            progress,
            world_coord,
        })
    }
}
