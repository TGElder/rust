use super::*;

use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;

const CHILDBEARING_AGE_START: f64 = 1_000_000.0 * 60.0 * 60.0 * 24.0 * 5.0;
const CHILDBEARING_AGE_END: f64 = 1_000_000.0 * 60.0 * 60.0 * 24.0 * 10.0;
const EXPECTED_CHILDREN: f64 = 2.0;

pub struct ChildrenParams {
    childbearing_age_start: u128,
    childbearing_age_end: u128,
    probability_per_microsecond: f64,
}

impl ChildrenParams {
    fn child_probability(
        &self,
        birthday: &u128,
        start_millis_exclusive: &u128,
        end_millis_inclusive: &u128,
    ) -> f64 {
        let childbearing_start: u128 = birthday + self.childbearing_age_start;
        let childbearing_end: u128 = birthday + self.childbearing_age_end;
        let start_millis = start_millis_exclusive.max(&childbearing_start);
        let end_millis = end_millis_inclusive.min(&childbearing_end);
        if end_millis <= start_millis {
            0.0
        } else {
            (end_millis - start_millis) as f64 * self.probability_per_microsecond
        }
    }
}

pub struct Children<R>
where
    R: Rng,
{
    command_tx: Sender<GameCommand>,
    params: ChildrenParams,
    rng: R,
}

impl Children<SmallRng> {
    pub fn new(command_tx: Sender<GameCommand>) -> Children<SmallRng> {
        Children {
            command_tx,
            params: ChildrenParams {
                childbearing_age_start: CHILDBEARING_AGE_START.round() as u128,
                childbearing_age_end: CHILDBEARING_AGE_END.round() as u128,
                probability_per_microsecond: EXPECTED_CHILDREN
                    / (CHILDBEARING_AGE_END - CHILDBEARING_AGE_START),
            },
            rng: SmallRng::from_rng(commons::rand::thread_rng()).unwrap(),
        }
    }

    fn add_children(
        &mut self,
        game_state: &GameState,
        start_millis_exclusive: &u128,
        end_millis_inclusive: &u128,
    ) {
        for avatar in game_state.avatars.values() {
            let position = if let Some(farm) = avatar.farm {
                farm
            } else {
                continue;
            };
            let r: f64 = self.rng.gen_range(0.0, 1.0);
            let p = self.params.child_probability(
                &avatar.birthday,
                start_millis_exclusive,
                end_millis_inclusive,
            );
            if r > p {
                continue;
            }
            let avatar_name = avatar.name.clone();
            let function: Box<dyn FnOnce(&mut GameState) -> Vec<GameCommand> + Send> =
                Box::new(move |game_state| {
                    let child_name = game_state.avatars.len().to_string();
                    let child = Avatar {
                        name: child_name.clone(),
                        birthday: game_state.game_micros,
                        state: AvatarState::Stationary {
                            position,
                            rotation: Rotation::Up,
                        },
                        farm: None,
                        children: vec![],
                    };
                    game_state.avatars.insert(child_name.clone(), child);
                    let children = &mut game_state.avatars.get_mut(&avatar_name).unwrap().children;
                    children.push(child_name);
                    vec![]
                });
            self.command_tx.send(GameCommand::Update(function)).unwrap();
        }
    }
}

impl GameEventConsumer for Children<SmallRng> {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Tick {
            start_millis_exclusive,
            end_millis_inclusive,
        } = event
        {
            self.add_children(game_state, start_millis_exclusive, end_millis_inclusive);
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
    use commons::Almost;

    #[test]
    fn child_probability_window_inside_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start: 100,
            childbearing_age_end: 200,
            probability_per_microsecond: 1.0,
        }
        .child_probability(&0, &120, &130)
        .almost(10.0));
    }
    #[test]
    fn child_probability_window_inside_childbearing_range_non_zero_birthday() {
        assert!(ChildrenParams {
            childbearing_age_start: 100,
            childbearing_age_end: 200,
            probability_per_microsecond: 1.0,
        }
        .child_probability(&100, &220, &230)
        .almost(10.0));
    }
    #[test]
    fn child_probability_window_contains_start_of_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start: 100,
            childbearing_age_end: 200,
            probability_per_microsecond: 1.0,
        }
        .child_probability(&0, &90, &110)
        .almost(10.0));
    }
    #[test]
    fn child_probability_window_contains_end_of_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start: 100,
            childbearing_age_end: 200,
            probability_per_microsecond: 1.0,
        }
        .child_probability(&0, &190, &210)
        .almost(10.0));
    }
    #[test]
    fn child_probability_window_before_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start: 100,
            childbearing_age_end: 200,
            probability_per_microsecond: 1.0,
        }
        .child_probability(&0, &80, &90)
        .almost(0.0));
    }
    #[test]
    fn child_probability_window_after_childbearing_range() {
        assert!(ChildrenParams {
            childbearing_age_start: 100,
            childbearing_age_end: 200,
            probability_per_microsecond: 1.0,
        }
        .child_probability(&0, &210, &220)
        .almost(0.0));
    }

    #[test]
    fn child_probability_childbearing_range_inside_window() {
        assert!(ChildrenParams {
            childbearing_age_start: 100,
            childbearing_age_end: 200,
            probability_per_microsecond: 1.0,
        }
        .child_probability(&0, &90, &210)
        .almost(100.0));
    }
}
