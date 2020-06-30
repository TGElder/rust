use super::*;
use crate::game::traits::Routes;
use crate::route::{Route, RouteKey};
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
        let (key, route) = match instruction {
            Instruction::Route { key, route } => (*key, route.clone()),
            _ => return state,
        };
        if let Some(instruction) = self
            .set_route(key, route.clone())
            .await
            .get_instruction(&key, &route)
        {
            println!("{} has changed", key);
            state.instructions.push(instruction);
        }
        State { ..state }
    }

    async fn set_route(&mut self, key: RouteKey, route: Route) -> SetRouteResult {
        self.game
            .update(move |routes| set_route(routes, key, route))
            .await
    }
}

fn set_route(routes: &mut dyn Routes, key: RouteKey, route: Route) -> SetRouteResult {
    match routes.routes_mut().entry(key) {
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

enum SetRouteResult {
    Add,
    Replace(Route),
    NoChange,
}

impl SetRouteResult {
    fn get_instruction(&self, key: &RouteKey, route: &Route) -> Option<Instruction> {
        match self {
            Self::Add => Self::get_route_changed_for_add(key, route),
            Self::Replace(old_route) => Self::get_route_changed_for_replace(key, route, old_route),
            Self::NoChange => None,
        }
    }

    fn get_route_changed_for_add(key: &RouteKey, new_route: &Route) -> Option<Instruction> {
        let positions_added: HashSet<V2<usize>> = new_route.path.iter().cloned().collect();
        Some(Instruction::RouteChanged {
            key: *key,
            positions_added,
            positions_removed: HashSet::new(),
        })
    }

    fn get_route_changed_for_replace(
        key: &RouteKey,
        new_route: &Route,
        old_route: &Route,
    ) -> Option<Instruction> {
        let new_hash_set: HashSet<&V2<usize>> = new_route.path.iter().collect();
        let old_hash_set: HashSet<&V2<usize>> = old_route.path.iter().collect();
        Some(Instruction::RouteChanged {
            key: *key,
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
        let key = RouteKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let state = block_on(async {
            processor
                .process(
                    State::default(),
                    &Instruction::Route {
                        key,
                        route: route.clone(),
                    },
                )
                .await
        });

        assert_eq!(
            state.instructions[0],
            Instruction::RouteChanged {
                key,
                positions_added: [v2(1, 3), v2(1, 4), v2(1, 5)].iter().cloned().collect(),
                positions_removed: HashSet::new(),
            }
        );
        let routes = handle.join().unwrap();
        assert_eq!(routes, vec![(key, route)].into_iter().collect())
    }

    #[test]
    fn should_add_route_and_new_route_instruction_if_route_has_changed() {
        let (game, mut rx) = update_channel(100);

        let handle = thread::spawn(move || {
            let mut routes = HashMap::new();
            routes.insert(
                RouteKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal,
                    destination: v2(1, 5),
                },
                Route {
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
        let key = RouteKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let state = block_on(async {
            processor
                .process(
                    State::default(),
                    &Instruction::Route {
                        key,
                        route: route.clone(),
                    },
                )
                .await
        });

        assert_eq!(
            state.instructions[0],
            Instruction::RouteChanged {
                key,
                positions_added: [v2(1, 4)].iter().cloned().collect(),
                positions_removed: [v2(2, 3), v2(2, 4), v2(2, 5)].iter().cloned().collect(),
            }
        );
        let routes = handle.join().unwrap();
        assert_eq!(routes, vec![(key, route)].into_iter().collect())
    }

    #[test]
    fn should_not_add_route_nor_new_route_instruction_if_route_is_unchanged() {
        let (game, mut rx) = update_channel(100);

        let key = RouteKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let route_2 = route.clone();

        let handle = thread::spawn(move || {
            let mut routes = HashMap::new();
            routes.insert(key, route_2);
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
                .process(
                    State::default(),
                    &Instruction::Route {
                        key,
                        route: route.clone(),
                    },
                )
                .await
        });

        assert_eq!(state.instructions, vec![],);
        let routes = handle.join().unwrap();
        assert_eq!(routes, vec![(key, route)].into_iter().collect())
    }
}
