use crate::pathfinder::Pathfinder;
use crate::traits::SendWorld;
use crate::travel_duration::TravelDuration;

pub struct PathfinderService<T, D>
where
    D: TravelDuration,
{
    tx: T,
    pathfinder: Option<Pathfinder<D>>,
}

impl<T, D> PathfinderService<T, D>
where
    T: SendWorld,
    D: TravelDuration + 'static,
{
    pub fn new(tx: T, pathfinder: Pathfinder<D>) -> PathfinderService<T, D> {
        PathfinderService {
            tx,
            pathfinder: Some(pathfinder),
        }
    }

    pub async fn init(&mut self) {
        let mut pathfinder = self.pathfinder.take().unwrap();

        let pathfinder = self
            .tx
            .send_world(move |world| {
                pathfinder.reset_edges(world);
                pathfinder
            })
            .await;

        self.pathfinder = Some(pathfinder);
    }

    pub fn pathfinder(&mut self) -> &mut Pathfinder<D> {
        self.pathfinder.as_mut().unwrap()
    }
}
