use super::*;

use crate::game::traits::HasWorld;
use crate::game::{CaptureEvent, GameEvent, GameEventConsumer, GameState};
use crate::pathfinder::traits::ClosestTargets;
use crate::world::{Resource, World, RESOURCES};
use commons::grid::Grid;
use commons::v2;
use isometric::Event;
use std::collections::HashSet;

const HANDLE: &str = "resource_route_targets";

pub struct ResourceTargets<G, P>
where
    P: ClosestTargets + Send + Sync,
    G: HasWorld,
{
    game: UpdateSender<G>,
    pathfinder: Arc<RwLock<P>>,
}

impl<G, P> ResourceTargets<G, P>
where
    P: ClosestTargets + Send + Sync,
    G: HasWorld,
{
    pub fn new(game: &UpdateSender<G>, pathfinder: &Arc<RwLock<P>>) -> ResourceTargets<G, P> {
        ResourceTargets {
            game: game.clone(),
            pathfinder: pathfinder.clone(),
        }
    }

    fn init(&mut self) {
        for resource in RESOURCES.iter() {
            self.init_resource(*resource);
        }
    }

    fn init_resource(&mut self, resource: Resource) {
        let targets = self.get_targets(resource);
        self.load_targets(target_set(resource), targets);
    }

    fn load_targets(&self, target_set: String, targets: HashSet<V2<usize>>) {
        let mut pathfinder = self.pathfinder.write().unwrap();
        pathfinder.init_targets(target_set.clone());
        for target in targets {
            pathfinder.load_target(&target_set, &target, true)
        }
    }

    fn get_targets(&mut self, resource: Resource) -> HashSet<V2<usize>> {
        block_on(async {
            self.game
                .update(move |game| get_targets(game, resource))
                .await
        })
    }
}

fn get_targets(game: &dyn HasWorld, resource: Resource) -> HashSet<V2<usize>> {
    let mut out = HashSet::new();
    let world = game.world();
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = &v2(x, y);
            if resource_at(&world, resource, &position) {
                out.insert(*position);
            }
        }
    }
    out
}

fn resource_at(world: &World, resource: Resource, position: &V2<usize>) -> bool {
    match world.get_cell(position) {
        Some(cell) if cell.resource == resource => true,
        _ => false,
    }
}

pub fn target_set(resource: Resource) -> String {
    format!("resource-{}", resource.name())
}

impl<G, P> GameEventConsumer for ResourceTargets<G, P>
where
    P: ClosestTargets + Send + Sync,
    G: HasWorld,
{
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Init = event {
            self.init();
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::pathfinder::traits::ClosestTargetResult;
    use commons::update::{process_updates, update_channel};
    use commons::{v2, M};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    fn world() -> World {
        World::new(M::zeros(3, 3), 0.5)
    }

    fn game_state() -> GameState {
        let world = world();
        GameState {
            world,
            ..GameState::default()
        }
    }

    struct MockPathfinder {
        targets: HashMap<String, M<bool>>,
    }

    impl ClosestTargets for MockPathfinder {
        fn init_targets(&mut self, name: String) {
            self.targets.insert(name, M::from_element(3, 3, false));
        }

        fn load_target(&mut self, name: &str, position: &V2<usize>, target: bool) {
            *self
                .targets
                .get_mut(name)
                .unwrap()
                .mut_cell_unsafe(position) = target;
        }

        fn closest_targets(&self, _: &[V2<usize>], _: &str, _: usize) -> Vec<ClosestTargetResult> {
            vec![]
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test() {
        let (game, mut rx) = update_channel(100);
        let running = Arc::new(AtomicBool::new(true));
        let running_2 = running.clone();
        let handle = thread::spawn(move || {
            let mut world = world();
            world.mut_cell_unsafe(&v2(1, 0)).resource = Resource::Coal;
            world.mut_cell_unsafe(&v2(2, 1)).resource = Resource::Coal;
            world.mut_cell_unsafe(&v2(0, 2)).resource = Resource::Coal;
            while running_2.load(Ordering::Relaxed) {
                let updates = rx.get_updates();
                process_updates(updates, &mut world);
            }
        });

        let pathfinder = Arc::new(RwLock::new(MockPathfinder {
            targets: HashMap::new(),
        }));

        let mut consumer = ResourceTargets::new(&game, &pathfinder);
        consumer.consume_game_event(&game_state(), &GameEvent::Init);
        running.store(false, Ordering::Relaxed);

        handle.join().unwrap();

        assert_eq!(
            *pathfinder
                .read()
                .unwrap()
                .targets
                .get("resource-coal")
                .unwrap(),
            M::from_vec(
                3,
                3,
                vec![
                    false, true, false,
                    false, false, true,
                    true, false, false,
                ]
            ),
        );
        assert_eq!(
            *pathfinder
                .read()
                .unwrap()
                .targets
                .get("resource-crops")
                .unwrap(),
            M::from_element(3, 3, false),
        );
    }
}
