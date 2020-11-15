use super::*;

use crate::game::traits::SetTerritory;
use crate::pathfinder::traits::PositionsWithin;
use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use commons::get_corners;
use commons::V2;
use std::sync::RwLock;
use std::time::Duration;

const NAME: &str = "territory_updater";

#[async_trait]
pub trait UpdateTerritory: Clone {
    async fn update_territory(&mut self, controller: V2<usize>);
}

pub struct TerritoryUpdater<G, P>
where
    G: SetTerritory + Send,
    P: PositionsWithin,
{
    game: FnSender<G>,
    pathfinder: Arc<RwLock<P>>,
    duration: Duration,
}

impl<G, P> TerritoryUpdater<G, P>
where
    G: SetTerritory + Send,
    P: PositionsWithin,
{
    pub fn new(
        game: &FnSender<G>,
        pathfinder: &Arc<RwLock<P>>,
        duration: Duration,
    ) -> TerritoryUpdater<G, P> {
        TerritoryUpdater {
            game: game.clone_with_name(NAME),
            pathfinder: pathfinder.clone(),
            duration,
        }
    }
}

impl<G, P> Clone for TerritoryUpdater<G, P>
where
    G: SetTerritory + Send,
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

#[async_trait]
impl<G, P> UpdateTerritory for TerritoryUpdater<G, P>
where
    G: SetTerritory + Send,
    P: PositionsWithin + Send + Sync,
{
    async fn update_territory(&mut self, controller: V2<usize>) {
        let states = vec![TerritoryState {
            controller,
            durations: self.get_durations(controller),
        }];
        self.set_territory(states).await;
    }
}

impl<G, P> TerritoryUpdater<G, P>
where
    G: SetTerritory + Send,
    P: PositionsWithin + Send + Sync,
{
    fn get_durations(&mut self, controller: V2<usize>) -> HashMap<V2<usize>, Duration> {
        let corners = get_corners(&controller);
        self.pathfinder
            .read()
            .unwrap()
            .positions_within(&corners, self.duration)
    }

    async fn set_territory(&mut self, states: Vec<TerritoryState>) {
        self.game.send(move |game| game.set_territory(states)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::game::TerritoryState;
    use commons::fn_sender::FnThread;
    use commons::futures::executor::block_on;
    use commons::same_elements;
    use commons::v2;
    use std::sync::Mutex;

    #[async_trait]
    impl UpdateTerritory for Arc<Mutex<Vec<V2<usize>>>> {
        async fn update_territory(&mut self, controller: V2<usize>) {
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
        let game = FnThread::new(vec![]);
        let pathfinder = Arc::new(RwLock::new(MockPathfinder {}));
        let duration = Duration::from_secs(5);

        let mut update_territory = TerritoryUpdater::new(&game.tx(), &pathfinder, duration);

        // When
        block_on(update_territory.update_territory(v2(1, 2)));
        let updates = game.join();

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
