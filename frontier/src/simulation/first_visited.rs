use super::*;

const HANDLE: &str = "first_visited_sim";
const BATCH_SIZE: usize = 128;

pub struct FirstVisitedSim {
    game_tx: UpdateSender<Game>,
}

impl Step for FirstVisitedSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {}

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl FirstVisitedSim {
    pub fn new(game_tx: &UpdateSender<Game>) -> FirstVisitedSim {
        FirstVisitedSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let start_at = self.get_game_micros().await;
        let routes = self.get_routes().await;
        for batch in routes.chunks(BATCH_SIZE) {
            self.update_first_visited_for_routes(start_at, batch.to_vec())
                .await;
        }
    }

    async fn get_game_micros(&mut self) -> u128 {
        self.game_tx.update(|game| get_game_micros(game)).await
    }

    async fn get_routes(&mut self) -> Vec<String> {
        self.game_tx.update(|game| get_routes(game)).await
    }

    async fn update_first_visited_for_routes(&mut self, start_at: u128, routes: Vec<String>) {
        self.game_tx
            .update(move |game| update_first_visited_for_routes(game, start_at, routes))
            .await;
    }
}

fn get_game_micros(game: &mut Game) -> u128 {
    game.game_state().game_micros
}

fn get_routes(game: &Game) -> Vec<String> {
    game.game_state().routes.keys().cloned().collect()
}

fn update_first_visited_for_routes(game: &mut Game, start_at: u128, routes: Vec<String>) {
    for route in routes {
        update_first_visited_for_route(game, start_at, route);
    }
}

fn update_first_visited_for_route(game: &mut Game, start_at: u128, route: String) {
    let route = match game.game_state().routes.get(&route) {
        Some(route) => route,
        None => return,
    };
    let first_visited = start_at + route.duration.as_micros();
    for position in route.path.clone() {
        update_first_visited_if_required(game, &position, first_visited);
    }
}

fn update_first_visited_if_required(game: &mut Game, position: &V2<usize>, first_visited: u128) {
    let maybe_first_visited = match game.mut_state().first_visited.get_mut(position) {
        Ok(maybe_first_visited) => maybe_first_visited,
        Err(..) => return,
    };
    match maybe_first_visited {
        None => *maybe_first_visited = Some(first_visited),
        Some(current_first_visited) if first_visited < *current_first_visited => {
            *maybe_first_visited = Some(first_visited)
        }
        _ => (),
    };
}
