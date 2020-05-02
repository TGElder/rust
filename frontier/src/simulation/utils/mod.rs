use super::*;

pub fn get_port_positions<'a>(
    game: &'a Game,
    path: &'a [V2<usize>],
) -> impl Iterator<Item = V2<usize>> + 'a {
    let travel_mode_fn = TravelModeFn::new(
        game.game_state()
            .params
            .avatar_travel
            .min_navigable_river_width,
    );
    let world = &game.game_state().world;
    path.edges()
        .flat_map(move |edge| travel_mode_fn.check_for_port(world, edge.from(), edge.to()))
}
