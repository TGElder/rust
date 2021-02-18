use commons::v2;

use crate::traits::has::HasParameters;
use crate::traits::UpdatePositionsAllPathfinders;

pub struct SetupPathfinders<T>
where
    T: HasParameters + UpdatePositionsAllPathfinders,
{
    tx: T,
}

impl<T> SetupPathfinders<T>
where
    T: HasParameters + UpdatePositionsAllPathfinders,
{
    pub fn new(tx: T) -> SetupPathfinders<T> {
        SetupPathfinders { tx }
    }

    pub async fn init(&mut self) {
        let width = self.tx.parameters().width;

        let all_positions = (0..width).flat_map(move |x| (0..width).map(move |y| v2(x, y)));

        self.tx
            .update_positions_all_pathfinders(all_positions)
            .await;
    }
}
