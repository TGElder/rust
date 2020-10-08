use super::*;

use crate::game::traits::SetTerritory;
use crate::pathfinder::traits::PositionsWithin;
use commons::futures::executor::block_on;
use commons::grid::get_corners;
use commons::update::UpdateSender;
use commons::V2;
use std::sync::RwLock;
use std::time::Duration;

const HANDLE: &str = "territory_updater";

pub trait UpdateTerritory: Clone {
    fn update_territory(&mut self, controller: V2<usize>);
}

pub struct TerritoryUpdater<G, P>
where
    G: SetTerritory,
    P: PositionsWithin,
{
    game: UpdateSender<G>,
    pathfinder: Arc<RwLock<P>>,
    duration: Duration,
}

impl<G, P> TerritoryUpdater<G, P>
where
    G: SetTerritory,
    P: PositionsWithin,
{
    pub fn new(
        game: &UpdateSender<G>,
        pathfinder: &Arc<RwLock<P>>,
        duration: Duration,
    ) -> TerritoryUpdater<G, P> {
        TerritoryUpdater {
            game: game.clone_with_handle(HANDLE),
            pathfinder: pathfinder.clone(),
            duration,
        }
    }
}

impl<G, P> Clone for TerritoryUpdater<G, P>
where
    G: SetTerritory,
    P: PositionsWithin,
{
    fn clone(&self) -> Self {
        TerritoryUpdater {
            game: self.game.clone(),
            pathfinder: self.pathfinder.clone(),
            duration: self.duration,
        }
    }
}

impl<G, P> UpdateTerritory for TerritoryUpdater<G, P>
where
    G: SetTerritory,
    P: PositionsWithin,
{
    fn update_territory(&mut self, controller: V2<usize>) {
        let states = vec![TerritoryState {
            controller,
            durations: self.get_durations(controller),
        }];
        self.set_territory(states);
    }
}

impl<G, P> TerritoryUpdater<G, P>
where
    G: SetTerritory,
    P: PositionsWithin,
{
    fn get_durations(&mut self, controller: V2<usize>) -> HashMap<V2<usize>, Duration> {
        let corners = get_corners(&controller);
        self.pathfinder
            .read()
            .unwrap()
            .positions_within(&corners, self.duration)
    }

    fn set_territory(&mut self, states: Vec<TerritoryState>) {
        block_on(self.game.update(move |game| game.set_territory(states)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::game::TerritoryState;
    use commons::same_elements;
    use commons::update::UpdateProcess;
    use commons::v2;
    use std::sync::Mutex;

    impl UpdateTerritory for Arc<Mutex<Vec<V2<usize>>>> {
        fn update_territory(&mut self, controller: V2<usize>) {
            self.lock().unwrap().push(controller);
        }
    }

    struct MockPathfinder {}

    impl PositionsWithin for MockPathfinder {
        fn positions_within(
            &self,
            positions: &[V2<usize>],
            duration: Duration,
        ) -> HashMap<V2<usize>, Duration> {
            assert!(same_elements(positions, &get_corners(&v2(1, 2))));
            assert_eq!(duration, Duration::from_secs(5));
            Self::durations()
        }
    }

    impl MockPathfinder {
        fn durations() -> HashMap<V2<usize>, Duration> {
            vec![
                (v2(1, 2), Duration::from_secs(0)),
                (v2(3, 4), Duration::from_secs(0)),
                (v2(1, 3), Duration::from_secs(1)),
                (v2(3, 5), Duration::from_secs(1)),
            ]
            .into_iter()
            .collect()
        }
    }

    #[test]
    fn test() {
        // Given
        let game = UpdateProcess::new(vec![]);
        let pathfinder = Arc::new(RwLock::new(MockPathfinder {}));
        let duration = Duration::from_secs(5);

        let mut update_territory = TerritoryUpdater::new(&game.tx(), &pathfinder, duration);

        // When
        update_territory.update_territory(v2(1, 2));
        let updates = game.shutdown();

        // Then
        assert_eq!(
            updates,
            vec![TerritoryState {
                controller: v2(1, 2),
                durations: MockPathfinder::durations(),
            }]
        );
    }
}
