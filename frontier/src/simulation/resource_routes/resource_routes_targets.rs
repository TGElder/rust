use super::*;

use commons::grid::{get_corners, Grid};
use std::collections::HashSet;

const HANDLE: &str = "resource_route_targets";

pub struct ResourceRouteTargets {
    pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
}

impl ResourceRouteTargets {
    pub fn new(
        pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
    ) -> ResourceRouteTargets {
        ResourceRouteTargets {
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        for resource in RESOURCES.iter() {
            self.init_resource(game_state, *resource);
        }
    }

    fn init_resource(&mut self, game_state: &GameState, resource: Resource) {
        let targets = get_targets(game_state, resource);
        block_on(self.load_targets(target_set(resource), targets));
    }

    async fn load_targets(&mut self, target_set: String, targets: HashSet<V2<usize>>) {
        self.pathfinder_tx
            .update(move |service| load_targets(&mut service.pathfinder(), target_set, targets));
    }
}

fn get_targets(game_state: &GameState, resource: Resource) -> HashSet<V2<usize>> {
    let mut out = HashSet::new();
    for x in 0..game_state.world.width() {
        for y in 0..game_state.world.height() {
            let position = &v2(x, y);
            get_corners(&position)
                .drain(..)
                .filter(|corner| resource_at(game_state, resource, &corner))
                .for_each(|corner| {
                    out.insert(corner);
                });
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

fn load_targets(
    pathfinder: &mut Pathfinder<AvatarTravelDuration>,
    target_set: String,
    targets: HashSet<V2<usize>>,
) {
    pathfinder.init_targets(target_set.clone());
    for target in targets {
        pathfinder.load_target(&target_set, &target, true)
    }
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
