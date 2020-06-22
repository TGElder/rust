use super::*;

use commons::grid::Grid;
use std::collections::HashSet;
use std::sync::RwLock;

const HANDLE: &str = "resource_route_targets";

pub struct ResourceRouteTargets {
    pathfinder: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
}

impl ResourceRouteTargets {
    pub fn new(pathfinder: &Arc<RwLock<Pathfinder<AvatarTravelDuration>>>) -> ResourceRouteTargets {
        ResourceRouteTargets {
            pathfinder: pathfinder.clone(),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        for resource in RESOURCES.iter() {
            self.init_resource(game_state, *resource);
        }
    }

    fn init_resource(&mut self, game_state: &GameState, resource: Resource) {
        let targets = get_targets(game_state, resource);
        self.load_targets(target_set(resource), targets);
    }

    fn load_targets(&self, target_set: String, targets: HashSet<V2<usize>>) {
        let mut pathfinder = self.pathfinder.write().unwrap();
        pathfinder.init_targets(target_set.clone());
        for target in targets {
            pathfinder.load_target(&target_set, &target, true)
        }
    }
}

fn get_targets(game_state: &GameState, resource: Resource) -> HashSet<V2<usize>> {
    let mut out = HashSet::new();
    for x in 0..game_state.world.width() {
        for y in 0..game_state.world.height() {
            let position = &v2(x, y);
            if resource_at(game_state, resource, &position) {
                out.insert(*position);
            }
        }
    }
    out
}

fn resource_at(game_state: &GameState, resource: Resource, position: &V2<usize>) -> bool {
    match game_state.world.get_cell(position) {
        Some(cell) if cell.resource == resource => true,
        _ => false,
    }
}

pub fn target_set(resource: Resource) -> String {
    format!("resource-{}", resource.name())
}

impl GameEventConsumer for ResourceRouteTargets {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Init = event {
            self.init(game_state);
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
