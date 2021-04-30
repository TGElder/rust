use crate::avatar::AvatarTravelDuration;

pub trait HasTravelDurations {
    fn npc_display_travel_duration(&self) -> &AvatarTravelDuration;
}
