use super::*;
use crate::avatar::*;
use crate::pathfinder::*;
use crate::territory::*;
use std::time::Duration;

pub struct TerritoryHandler {
    command_tx: Sender<GameCommand>,
    pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    duration: Duration,
}

impl TerritoryHandler {
    pub fn new(
        command_tx: Sender<GameCommand>,
        pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
        duration: Duration,
    ) -> TerritoryHandler {
        TerritoryHandler {
            command_tx,
            pathfinder_tx,
            duration,
        }
    }

    fn remove_controller(&self, controller: V2<usize>) {
        self.command_tx
            .send(GameCommand::SetTerritory(vec![TerritoryState {
                controller,
                territory: HashSet::new(),
            }]))
            .unwrap();
    }

    fn update_controllers(&self, world: &World, controllers: Vec<V2<usize>>) {
        let controllers: Vec<(V2<usize>, [V2<usize>; 4])> = controllers
            .iter()
            .map(|controller| (*controller, world.get_corners(&controller)))
            .collect();
        let duration = self.duration;
        let function: Box<dyn Fn(&Pathfinder<AvatarTravelDuration>) -> Vec<GameCommand> + Send> =
            Box::new(move |pathfinder| {
                let mut states = vec![];
                for (controller, corners) in controllers.iter() {
                    let territory = pathfinder
                        .positions_within(corners, duration)
                        .iter()
                        .cloned()
                        .collect();
                    states.push(TerritoryState {
                        controller: *controller,
                        territory,
                    });
                }
                return vec![GameCommand::SetTerritory(states)];
            });
        self.pathfinder_tx
            .send(PathfinderCommand::Use(function))
            .unwrap();
    }

    fn update_positions(&self, world: &World, territory: &Territory, positions: &[V2<usize>]) {
        let controllers: HashSet<V2<usize>> = positions
            .iter()
            .flat_map(|position| territory.who_controls(position))
            .cloned()
            .collect();

        self.update_controllers(world, controllers.iter().cloned().collect())
    }

    fn update_all(&self, world: &World, territory: &Territory) {
        let controllers = territory
            .controllers()
            .map(|(controller, _)| controller)
            .cloned()
            .collect();
        self.update_controllers(world, controllers);
    }
}

impl GameEventConsumer for TerritoryHandler {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::HouseUpdated { position, built } => match built {
                true => self.update_controllers(&game_state.world, vec![*position]),
                false => self.remove_controller(*position),
            },
            GameEvent::RoadsUpdated(result) => {
                self.update_positions(&game_state.world, &game_state.territory, result.path())
            }
            GameEvent::CellsRevealed(CellSelection::Some(positions)) => {
                self.update_positions(&game_state.world, &game_state.territory, positions)
            }
            GameEvent::CellsRevealed(CellSelection::All) => {
                self.update_all(&game_state.world, &game_state.territory)
            }
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
