use super::*;
use crate::game::traits::Routes;
use crate::route::Route;
use std::collections::hash_map::Entry;

const HANDLE: &str = "new_route_to_route";

pub struct NewRouteToRoute<G>
where
    G: Routes,
{
    game: UpdateSender<G>,
}

impl<G> Processor for NewRouteToRoute<G>
where
    G: Routes,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        block_on(self.process(state, instruction))
    }
}

impl<G> NewRouteToRoute<G>
where
    G: Routes,
{
    pub fn new(game: &UpdateSender<G>) -> NewRouteToRoute<G> {
        NewRouteToRoute {
            game: game.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let route = match instruction {
            Instruction::Route(route) => route.clone(),
            _ => return state,
        };
        if self.try_add_route(route.clone()).await {
            println!("{:?} has changed", route_name(&route));
            state.instructions.push(Instruction::NewRoute(route));
        }
        State { ..state }
    }

    async fn try_add_route(&mut self, route: Route) -> bool {
        self.game
            .update(move |routes| try_add_route(routes, route))
            .await
    }
}

fn try_add_route(routes: &mut dyn Routes, route: Route) -> bool {
    match routes.routes_mut().entry(route_name(&route)) {
        Entry::Occupied(mut entry) if *entry.get() != route => {
            entry.insert(route);
            true
        }
        Entry::Vacant(entry) => {
            entry.insert(route);
            true
        }
        _ => false,
    }
}

fn route_name(route: &Route) -> String {
    let destination = route.path.last().unwrap_or_else(|| {
        panic!(
            "Invalid route from {} for {:?} with empty path",
            route.settlement, route.resource
        )
    });
    format!(
        "({}, {})-{:?}-({}, {})",
        route.settlement.x, route.settlement.y, route.resource, destination.x, destination.y
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::world::Resource;
    use commons::update::{process_updates, update_channel};
    use commons::v2;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn should_add_route_and_new_route_instruction_if_route_is_new() {
        let (game, mut rx) = update_channel(100);

        let handle = thread::spawn(move || {
            let mut routes = HashMap::new();
            loop {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut routes);
                    return routes;
                }
            }
        });

        let mut processor = NewRouteToRoute::new(&game);
        let route = Route {
            resource: Resource::Coal,
            settlement: v2(1, 3),
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let state = block_on(async {
            processor
                .process(State::default(), &Instruction::Route(route.clone()))
                .await
        });
        
        assert_eq!(state.instructions[0], Instruction::NewRoute(route.clone()),);
        let routes = handle.join().unwrap();
        assert_eq!(
            routes,
            vec![(String::from("(1, 3)-Coal-(1, 5)"), route)]
                .into_iter()
                .collect()
        )
    }

    #[test]
    fn should_add_route_and_new_route_instruction_if_route_has_changed() {
        let (game, mut rx) = update_channel(100);

        let handle = thread::spawn(move || {
            let mut routes = HashMap::new();
            routes.insert(
                String::from("(1, 3)-Coal-(1, 5)"),
                Route {
                    resource: Resource::Coal,
                    settlement: v2(1, 3),
                    path: vec![v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)],
                    start_micros: 0,
                    duration: Duration::from_secs(4),
                    traffic: 3,
                },
            );
            loop {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut routes);
                    return routes;
                }
            }
        });

        let mut processor = NewRouteToRoute::new(&game);
        let route = Route {
            resource: Resource::Coal,
            settlement: v2(1, 3),
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let state = block_on(async {
            processor
                .process(State::default(), &Instruction::Route(route.clone()))
                .await
        });

        assert_eq!(state.instructions[0], Instruction::NewRoute(route.clone()),);
        let routes = handle.join().unwrap();
        assert_eq!(
            routes,
            vec![(String::from("(1, 3)-Coal-(1, 5)"), route)]
                .into_iter()
                .collect()
        )
    }

    #[test]
    fn should_not_add_route_nor_new_route_instruction_if_route_is_unchanged() {
        let (game, mut rx) = update_channel(100);

        let route = Route {
            resource: Resource::Coal,
            settlement: v2(1, 3),
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let route_2 = route.clone();

        let handle = thread::spawn(move || {
            let mut routes = HashMap::new();
            routes.insert(String::from("(1, 3)-Coal-(1, 5)"), route_2);
            loop {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut routes);
                    return routes;
                }
            }
        });

        let mut processor = NewRouteToRoute::new(&game);

        let state = block_on(async {
            processor
                .process(State::default(), &Instruction::Route(route.clone()))
                .await
        });
        
        assert_eq!(state.instructions, vec![],);
        let routes = handle.join().unwrap();
        assert_eq!(
            routes,
            vec![(String::from("(1, 3)-Coal-(1, 5)"), route)]
                .into_iter()
                .collect()
        )
    }
}
