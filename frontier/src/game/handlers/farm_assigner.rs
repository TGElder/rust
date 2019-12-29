use super::farm_candidate_handler::NAME as FARM_CANDIDATE_NAME;
use super::town_candidate_handler::NAME as TOWN_CANDIDATE_NAME;
use super::*;
use crate::avatar::*;
use crate::pathfinder::*;

pub struct FarmAssigner {
    pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>,
    done_tx: Sender<()>,
    done_rx: Receiver<()>,
    queue: Vec<String>,
}

impl FarmAssigner {
    pub fn new(pathfinder_tx: Sender<PathfinderCommand<AvatarTravelDuration>>) -> FarmAssigner {
        let (done_tx, done_rx) = mpsc::channel();
        done_tx.send(()).unwrap();
        FarmAssigner {
            pathfinder_tx,
            done_tx,
            done_rx,
            queue: vec![],
        }
    }

    fn run(&mut self, game_state: &GameState) {
        if let Ok(()) = self.done_rx.try_recv() {
            if self.queue.is_empty() {
                self.fill_queue(game_state);
            }
            if let Some(avatar) = self.queue.pop() {
                self.try_assign_farm(game_state, avatar);
            } else {
                self.done_tx.send(()).unwrap();
            }
        }
    }

    fn fill_queue(&mut self, game_state: &GameState) {
        self.queue = game_state
            .avatars
            .values()
            .filter(|avatar| avatar.farm == None)
            .map(|avatar| avatar.name.clone())
            .collect();
    }

    fn try_assign_farm(&mut self, game_state: &GameState, avatar: String) {
        if let Some(Avatar {
            state: AvatarState::Stationary { position, .. },
            farm: None,
            ..
        }) = game_state.avatars.get(&avatar)
        {
            let position = *position;
            let done_tx = self.done_tx.clone();
            let function: Box<
                dyn FnOnce(&Pathfinder<AvatarTravelDuration>) -> Vec<GameCommand> + Send,
            > = Box::new(move |pathfinder| {
                let mut towns = pathfinder.closest_targets(&[position], TOWN_CANDIDATE_NAME);
                let mut closest = match towns.pop() {
                    Some(home) => pathfinder.closest_targets(&[home], FARM_CANDIDATE_NAME),
                    None => pathfinder.closest_targets(&[position], FARM_CANDIDATE_NAME),
                };
                done_tx.send(()).unwrap();
                closest
                    .pop()
                    .map(|closest| {
                        vec![GameCommand::SetAvatarFarm {
                            name: avatar,
                            farm: Some(closest),
                        }]
                    })
                    .unwrap_or_default()
            });
            self.pathfinder_tx
                .send(PathfinderCommand::Use(function))
                .unwrap();
        } else {
            self.done_tx.send(()).unwrap();
        }
    }
}

impl GameEventConsumer for FarmAssigner {
    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Tick = *event {
            self.run(game_state);
        }
        CaptureEvent::No
    }
}
