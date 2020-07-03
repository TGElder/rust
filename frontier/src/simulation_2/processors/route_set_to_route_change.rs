use super::*;
use crate::game::traits::Routes;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

const HANDLE: &str = "route_set_to_route_change";

pub struct RouteSetToRouteChange<G>
where
    G: Routes,
{
    game: UpdateSender<G>,
}

impl<G> Processor for RouteSetToRouteChange<G>
where
    G: Routes,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        block_on(self.process(state, instruction))
    }
}

impl<G> RouteSetToRouteChange<G>
where
    G: Routes,
{
    pub fn new(game: &UpdateSender<G>) -> RouteSetToRouteChange<G> {
        RouteSetToRouteChange {
            game: game.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let (key, route_set) = match instruction {
            Instruction::RouteSet { key, route_set } => (key, route_set),
            _ => return state,
        };
        let state = self.process_new_and_changed(state, key, route_set).await;
        self.process_removed(state, key, route_set).await
    }

    pub async fn process_new_and_changed(
        &mut self,
        mut state: State,
        set_key: &RouteSetKey,
        route_set: &RouteSet,
    ) -> State {
        for (key, route) in route_set.iter() {
            if let Some(route_change) = self.set_route(*set_key, *key, route.clone()).await {
                log_change(&route_change);
                state
                    .instructions
                    .push(Instruction::RouteChange(route_change));
            }
        }
        state
    }

    pub async fn process_removed(
        &mut self,
        mut state: State,
        set_key: &RouteSetKey,
        route_set: &RouteSet,
    ) -> State {
        let removed = self
            .get_removed(*set_key, route_set.keys().cloned().collect())
            .await;

        for route_change in removed.into_iter() {
            log_change(&route_change);
            state
                .instructions
                .push(Instruction::RouteChange(route_change))
        }
        state
    }

    async fn set_route(
        &mut self,
        set_key: RouteSetKey,
        key: RouteKey,
        route: Route,
    ) -> Option<RouteChange> {
        self.game
            .update(move |routes| set_route(routes, set_key, key, route))
            .await
    }

    async fn get_removed(
        &mut self,
        set_key: RouteSetKey,
        keys: HashSet<RouteKey>,
    ) -> Vec<RouteChange> {
        self.game
            .update(move |routes| get_removed(routes, set_key, keys))
            .await
    }
}

fn log_change(route_change: &RouteChange) {
    match route_change {
        RouteChange::New { key, .. } => println!("{} was added", key),
        RouteChange::Updated { key, .. } => println!("{} was updated", key),
        RouteChange::Removed { key, .. } => println!("{} was removed", key),
    }
}

fn set_route(
    routes: &mut dyn Routes,
    set_key: RouteSetKey,
    key: RouteKey,
    route: Route,
) -> Option<RouteChange> {
    let route_set = routes
        .routes_mut()
        .entry(set_key)
        .or_insert_with(HashMap::new);
    match route_set.entry(key) {
        Entry::Occupied(mut entry) if *entry.get() != route => {
            let old = entry.insert(route.clone());
            Some(RouteChange::Updated {
                key,
                old,
                new: route,
            })
        }
        Entry::Vacant(entry) => {
            entry.insert(route.clone());
            Some(RouteChange::New { key, route })
        }
        _ => None,
    }
}

fn get_removed(
    routes: &mut dyn Routes,
    set_key: RouteSetKey,
    new_keys: HashSet<RouteKey>,
) -> Vec<RouteChange> {
    let route_set = routes
        .routes_mut()
        .entry(set_key)
        .or_insert_with(HashMap::new);
    let previous_keys: HashSet<RouteKey> = route_set.keys().cloned().collect();
    let to_remove = previous_keys.difference(&new_keys);
    let mut out = vec![];
    for key in to_remove {
        let route = route_set.remove(key).unwrap();
        out.push(RouteChange::Removed { key: *key, route });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::world::Resource;
    use commons::update::{process_updates, update_channel};
    use commons::v2;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn should_add_route_and_new_route_instruction_if_route_is_new() {
        let (game, mut rx) = update_channel(100);
        let run = Arc::new(AtomicBool::new(true));
        let run_2 = run.clone();
        let handle = thread::spawn(move || {
            let mut routes = HashMap::new();
            while run_2.load(Ordering::Relaxed) {
                let updates = rx.get_updates();
                process_updates(updates, &mut routes);
            }
            routes
        });

        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let mut route_set = HashMap::new();
        route_set.insert(key, route.clone());

        let mut processor = RouteSetToRouteChange::new(&game);
        let state = block_on(async {
            processor
                .process(
                    State::default(),
                    &Instruction::RouteSet {
                        key: set_key,
                        route_set,
                    },
                )
                .await
        });

        assert_eq!(
            state.instructions[0],
            Instruction::RouteChange(RouteChange::New {
                key,
                route: route.clone()
            })
        );

        run.store(false, Ordering::Relaxed);
        let routes = handle.join().unwrap();
        assert_eq!(
            routes,
            vec![(set_key, vec![(key, route)].into_iter().collect())]
                .into_iter()
                .collect()
        )
    }

    #[test]
    fn should_add_route_and_updated_route_instruction_if_route_has_changed() {
        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let old = Route {
            path: vec![v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(4),
            traffic: 3,
        };
        let new = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let mut route_set = HashMap::new();
        route_set.insert(key, new.clone());

        let old_2 = old.clone();
        let (game, mut rx) = update_channel(100);
        let run = Arc::new(AtomicBool::new(true));
        let run_2 = run.clone();
        let handle = thread::spawn(move || {
            let mut route_set = HashMap::new();
            route_set.insert(key, old_2);
            let mut routes = HashMap::new();
            routes.insert(set_key, route_set);
            while run_2.load(Ordering::Relaxed) {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut routes);
                }
            }
            routes
        });

        let mut processor = RouteSetToRouteChange::new(&game);
        let state = block_on(async {
            processor
                .process(
                    State::default(),
                    &Instruction::RouteSet {
                        key: set_key,
                        route_set,
                    },
                )
                .await
        });

        assert_eq!(
            state.instructions[0],
            Instruction::RouteChange(RouteChange::Updated {
                key,
                new: new.clone(),
                old
            })
        );

        run.store(false, Ordering::Relaxed);
        let routes = handle.join().unwrap();
        assert_eq!(
            routes,
            vec![(set_key, vec![(key, new)].into_iter().collect())]
                .into_iter()
                .collect()
        )
    }

    #[test]
    fn should_not_add_route_nor_instruction_if_route_is_unchanged() {
        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let mut route_set = HashMap::new();
        route_set.insert(key, route.clone());

        let route_2 = route.clone();
        let (game, mut rx) = update_channel(100);
        let run = Arc::new(AtomicBool::new(true));
        let run_2 = run.clone();
        let handle = thread::spawn(move || {
            let mut route_set = HashMap::new();
            route_set.insert(key, route_2);
            let mut routes = HashMap::new();
            routes.insert(set_key, route_set);
            while run_2.load(Ordering::Relaxed) {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut routes);
                }
            }
            routes
        });

        let mut processor = RouteSetToRouteChange::new(&game);
        let state = block_on(async {
            processor
                .process(
                    State::default(),
                    &Instruction::RouteSet {
                        key: set_key,
                        route_set,
                    },
                )
                .await
        });

        assert_eq!(state.instructions, vec![],);

        run.store(false, Ordering::Relaxed);
        let routes = handle.join().unwrap();
        assert_eq!(
            routes,
            vec![(set_key, vec![(key, route)].into_iter().collect())]
                .into_iter()
                .collect()
        )
    }

    #[test]
    fn should_remove_route_and_add_removed_route_instruction_if_route_is_removed() {
        let set_key = RouteSetKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
        };
        let key = RouteKey {
            settlement: set_key.settlement,
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let route = Route {
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        };
        let route_set = HashMap::new();

        let route_2 = route.clone();
        let (game, mut rx) = update_channel(100);
        let run = Arc::new(AtomicBool::new(true));
        let run_2 = run.clone();
        let handle = thread::spawn(move || {
            let mut route_set = HashMap::new();
            route_set.insert(key, route_2);
            let mut routes = HashMap::new();
            routes.insert(set_key, route_set);
            while run_2.load(Ordering::Relaxed) {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut routes);
                }
            }
            routes
        });

        let mut processor = RouteSetToRouteChange::new(&game);
        let state = block_on(async {
            processor
                .process(
                    State::default(),
                    &Instruction::RouteSet {
                        key: set_key,
                        route_set,
                    },
                )
                .await
        });

        assert_eq!(
            state.instructions[0],
            Instruction::RouteChange(RouteChange::Removed { key, route })
        );

        run.store(false, Ordering::Relaxed);
        let routes = handle.join().unwrap();
        assert_eq!(
            routes,
            vec![(set_key, HashMap::new())].into_iter().collect()
        )
    }
}
