use super::town_candidate_handler::NAME as TOWN_CANDIDATE_NAME;
use super::*;
use crate::avatar::*;
use crate::pathfinder::*;
use std::cmp::Reverse;

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
            self.pathfinder_tx
                .send(PathfinderCommand::Use(
                    Self::find_town_then_set_avatar_farm(avatar, *position, self.done_tx.clone()),
                ))
                .unwrap();
        } else {
            self.done_tx.send(()).unwrap();
        }
    }

    fn find_town_then_set_avatar_farm(
        avatar_name: String,
        position: V2<usize>,
        done_tx: Sender<()>,
    ) -> Box<dyn FnOnce(&Pathfinder<AvatarTravelDuration>) -> Vec<GameCommand> + Send> {
        Box::new(move |pathfinder| {
            let mut towns = pathfinder.closest_targets(&[position], TOWN_CANDIDATE_NAME);
            if let Some(town) = towns.pop() {
                return vec![GameCommand::Update(Self::set_avatar_farm(
                    avatar_name,
                    town,
                    done_tx,
                ))];
            }
            done_tx.send(()).unwrap();
            vec![]
        })
    }

    fn set_avatar_farm(
        avatar_name: String,
        town: V2<usize>,
        done_tx: Sender<()>,
    ) -> Box<dyn FnOnce(&mut GameState) -> Vec<GameCommand> + Send> {
        Box::new(move |game_state| {
            let mut farms: Vec<V2<usize>> = game_state
                .territory
                .controlled_tiles(&town)
                .into_iter()
                .filter(|position| game_state.is_farm_candidate(position))
                .collect();
            farms.sort_by_key(|a| {
                Reverse(
                    game_state
                        .territory
                        .get_claim(a, &town)
                        .map(|claim| claim.duration),
                )
            });
            done_tx.send(()).unwrap();
            farms
                .pop()
                .map(|farm| {
                    vec![GameCommand::SetAvatarFarm {
                        name: avatar_name,
                        farm: Some(farm),
                    }]
                })
                .unwrap_or_default()
        })
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
