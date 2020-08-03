use crate::avatar::{AvatarTravelModeFn, CheckForPort};
use crate::game::Game;
use crate::world::World;
use commons::V2;

impl CheckForPort for Game {
    fn check_for_port(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<V2<usize>> {
        let travel_mode_fn = AvatarTravelModeFn::new(
            self.game_state
                .params
                .avatar_travel
                .min_navigable_river_width,
        );
        travel_mode_fn.check_for_port(world, from, to)
    }
}
