use crate::avatar::AvatarTravelDuration;

pub trait HasTravelDurations {
    fn npc_travel_duration(&self) -> &AvatarTravelDuration;
}
