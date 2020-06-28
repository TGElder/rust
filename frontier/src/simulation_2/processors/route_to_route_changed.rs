use super::*;
use crate::game::traits::Routes;
use crate::route::Route;
use std::collections::hash_map::Entry;
use std::collections::HashSet;

const HANDLE: &str = "new_route_to_route";

pub struct RouteToRouteChanged<G>
where
    G: Routes,
{
    game: UpdateSender<G>,
}

impl<G> Processor for RouteToRouteChanged<G>
where
    G: Routes,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        block_on(self.process(state, instruction))
    }
}

impl<G> RouteToRouteChanged<G>
where
    G: Routes,
{
    pub fn new(game: &UpdateSender<G>) -> RouteToRouteChanged<G> {
        RouteToRouteChanged {
            game: game.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let route = match instruction {
            Instruction::Route(route) => route.clone(),
            _ => return state,
        };
        if let Some(instruction) = self
            .try_add_route(route.clone())
            .await
            .get_instruction(&route)
        {
            println!("{:?} has changed", route_name(&route));
            state.instructions.push(instruction);
        }
        State { ..state }
    }

    async fn try_add_route(&mut self, route: Route) -> SetRouteResult {
        self.game
            .update(move |routes| set_route(routes, route))
            .await
    }
}

fn set_route(routes: &mut dyn Routes, route: Route) -> SetRouteResult {
    match routes.routes_mut().entry(route_name(&route)) {
        Entry::Occupied(mut entry) if *entry.get() != route => {
            SetRouteResult::Replace(entry.insert(route))
        }
        Entry::Vacant(entry) => {
            entry.insert(route);
            SetRouteResult::Add
        }
        _ => SetRouteResult::NoChange,
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

enum SetRouteResult {
    Add,
    Replace(Route),
    NoChange,
}

impl SetRouteResult {
    fn get_instruction(&self, route: &Route) -> Option<Instruction> {
        match self {
            Self::Add => Self::get_route_changed_for_add(route),
            Self::Replace(old_route) => Self::get_route_changed_for_replace(route, old_route),
            Self::NoChange => None,
        }
    }

    fn get_route_changed_for_add(new_route: &Route) -> Option<Instruction> {
        let positions_added: HashSet<V2<usize>> = new_route.path.iter().cloned().collect();
        Some(Instruction::RouteChanged {
            new_route: new_route.clone(),
            positions_added,
            positions_removed: HashSet::new(),
        })
    }

    fn get_route_changed_for_replace(new_route: &Route, old_route: &Route) -> Option<Instruction> {
        let new_hash_set: HashSet<&V2<usize>> = new_route.path.iter().collect();
        let old_hash_set: HashSet<&V2<usize>> = old_route.path.iter().collect();
        Some(Instruction::RouteChanged {
            new_route: new_route.clone(),
            positions_added: new_hash_set
                .difference(&old_hash_set)
                .cloned()
                .cloned()
                .collect(),
            positions_removed: old_hash_set
                .difference(&new_hash_set)
                .cloned()
                .cloned()
                .collect(),
        })
    }
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

        let mut processor = RouteToRouteChanged::new(&game);
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

        assert_eq!(
            state.instructions[0],
            Instruction::RouteChanged {
                new_route: route.clone(),
                positions_added: [v2(1, 3), v2(1, 4), v2(1, 5)].iter().cloned().collect(),
                positions_removed: HashSet::new(),
            }
        );
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

        let mut processor = RouteToRouteChanged::new(&game);
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

        assert_eq!(
            state.instructions[0],
            Instruction::RouteChanged {
                new_route: route.clone(),
                positions_added: [v2(1, 4)].iter().cloned().collect(),
                positions_removed: [v2(2, 3), v2(2, 4), v2(2, 5)].iter().cloned().collect(),
            }
        );
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

        let mut processor = RouteToRouteChanged::new(&game);

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
