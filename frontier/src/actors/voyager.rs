use crate::game::GameState;
use crate::settlement::SettlementClass;
use crate::traits::{RevealCells, SendGame, SendWorld};
use crate::world::World;
use commons::async_channel::{Receiver, RecvError};
use commons::fn_sender::{FnMessage, FnMessageExt, FnReceiver};
use commons::futures::future::FutureExt;
use commons::{v2, Grid, V2};
use isometric::Event;
use line_drawing::WalkGrid;
use std::collections::HashSet;
use std::sync::Arc;

const NAME: &str = "voyager";

pub struct Voyager<T> {
    x: T,
    rx: FnReceiver<Voyager<T>>,
    engine_rx: Receiver<Arc<Event>>,
    run: bool,
}

impl<T> Voyager<T>
where
    T: RevealCells + SendGame + SendWorld + Send,
{
    pub fn new(x: T, rx: FnReceiver<Voyager<T>>, engine_rx: Receiver<Arc<Event>>) -> Voyager<T> {
        Voyager {
            x,
            rx,
            engine_rx,
            run: true,
        }
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                mut message = self.rx.get_message().fuse() => self.handle_message(message).await,
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event)
            }
        }
    }

    async fn handle_message(&mut self, mut message: FnMessage<Voyager<T>>) {
        message.apply(self).await;
    }

    pub async fn voyage_to(&mut self, cells: Vec<V2<usize>>, by: &'static str) {
        if by == NAME {
            return;
        } // avoid chain reaction
        let homeland_positions = self
            .x
            .send_game(|game| homeland_positions(game.game_state()))
            .await;
        let mut to_reveal = self
            .x
            .send_world(move |world| {
                let voyaged = get_all_voyaged(world, homeland_positions, cells);
                extend_all(world, &voyaged)
            })
            .await;
        self.reveal_cells(to_reveal.drain().collect()).await;
    }

    async fn reveal_cells(&mut self, revealed: Vec<V2<usize>>) {
        self.x.reveal_cells(revealed, NAME).await;
    }

    fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        if let Event::Shutdown = *event.unwrap() {
            self.shutdown();
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }
}

fn homeland_positions(game_state: &GameState) -> Vec<V2<usize>> {
    game_state
        .settlements
        .values()
        .filter(|settlement| settlement.class == SettlementClass::Homeland)
        .map(|homeland| homeland.position)
        .collect()
}

fn extend_all(world: &World, positions: &HashSet<V2<usize>>) -> HashSet<V2<usize>> {
    positions
        .iter()
        .flat_map(|position| world.expand_position(position))
        .collect()
}

fn get_all_voyaged(world: &World, from: Vec<V2<usize>>, to: Vec<V2<usize>>) -> HashSet<V2<usize>> {
    let mut out = HashSet::new();
    for from in from {
        for to in to.iter() {
            out.extend(unwrap_or!(get_voyage(world, &from, &to), continue));
        }
    }
    out
}

fn get_voyage(world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Vec<V2<usize>>> {
    if !world.get_cell_unsafe(&to).visible {
        return None;
    }
    if !is_coastal(world, to) {
        return None;
    }
    let mut voyage: Vec<V2<usize>> =
        WalkGrid::new((from.x as i32, from.y as i32), (to.x as i32, to.y as i32))
            .map(|(x, y)| v2(x as usize, y as usize))
            .collect();
    if !voyage
        .iter()
        .all(|position| world.is_sea(position) || !world.get_cell_unsafe(position).visible)
    {
        return None;
    }
    if !voyage
        .iter()
        .any(|position| !world.get_cell_unsafe(position).visible)
    {
        return None;
    }
    Some(
        voyage
            .drain(..)
            .take_while(|position| world.is_sea(&position))
            .collect(),
    )
}

fn is_coastal(world: &World, position: &V2<usize>) -> bool {
    if !world.is_sea(position) {
        return false;
    }
    if !world.get_cell_unsafe(position).visible {
        return false;
    }
    world
        .neighbours(position)
        .iter()
        .any(|position| !world.is_sea(position) && world.get_cell_unsafe(position).visible)
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::M;

    #[rustfmt::skip]
    fn world() -> World {

        World::new(
            M::from_vec(5, 5, vec![
                0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 1.0, 0.0,
                0.0, 1.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
            0.5
        )
        
    }

    #[test]
    fn test_is_coastal() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        assert!(is_coastal(&world, &v2(1, 0)));
    }

    #[test]
    fn test_not_coastal_if_land_invisible() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        assert!(!is_coastal(&world, &v2(1, 0)));
    }

    #[test]
    fn test_not_coastal_if_sea_invisible() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        assert!(!is_coastal(&world, &v2(1, 0)));
    }

    #[test]
    fn test_non_coast_sea_is_not_coastal() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(0, 0)).visible = true;
        assert!(!is_coastal(&world, &v2(0, 0)));
    }

    #[test]
    fn test_land_is_not_coastal() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        assert!(!is_coastal(&world, &v2(1, 1)));
    }

    #[test]
    fn test_get_voyage() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        world.mut_cell_unsafe(&v2(2, 1)).visible = true;
        assert_eq!(
            get_voyage(&world, &v2(4, 1), &v2(2, 1)),
            Some(vec![v2(4, 1), v2(3, 1), v2(2, 1)])
        );
    }

    #[test]
    fn test_from_visible_is_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        world.mut_cell_unsafe(&v2(2, 1)).visible = true;
        world.mut_cell_unsafe(&v2(4, 1)).visible = true;
        assert_eq!(
            get_voyage(&world, &v2(4, 1), &v2(2, 1)),
            Some(vec![v2(4, 1), v2(3, 1), v2(2, 1)])
        );
    }

    #[test]
    fn test_to_invisibile_not_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(4, 1)).visible = true;
        world.mut_cell_unsafe(&v2(3, 1)).visible = true;
        assert_eq!(get_voyage(&world, &v2(4, 1), &v2(2, 1)), None);
    }

    #[test]
    fn test_all_visibile_not_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        world.mut_cell_unsafe(&v2(2, 1)).visible = true;
        world.mut_cell_unsafe(&v2(3, 1)).visible = true;
        world.mut_cell_unsafe(&v2(4, 1)).visible = true;
        assert_eq!(get_voyage(&world, &v2(4, 1), &v2(2, 1)), None);
    }

    #[test]
    fn test_visible_land_in_way_not_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 0)).visible = true;
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        assert_eq!(get_voyage(&world, &v2(1, 4), &v2(1, 0)), None)
    }

    #[test]
    fn test_invisible_land_in_way_is_voyagable() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 2)).visible = true;
        world.mut_cell_unsafe(&v2(2, 2)).visible = true;
        assert_eq!(
            get_voyage(&world, &v2(4, 2), &v2(2, 2)),
            Some(vec![v2(4, 2)])
        );
    }

    #[test]
    fn test_to_not_coastal_not_voyagle() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(2, 4)).visible = true;
        assert_eq!(get_voyage(&world, &v2(4, 4), &v2(2, 4)), None)
    }

    #[test]
    fn test_get_all_voyaged() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(1, 1)).visible = true;
        world.mut_cell_unsafe(&v2(2, 1)).visible = true;
        world.mut_cell_unsafe(&v2(1, 3)).visible = true;
        world.mut_cell_unsafe(&v2(2, 3)).visible = true;

        let actual = get_all_voyaged(&world, vec![v2(4, 1), v2(4, 3)], vec![v2(2, 1), v2(2, 3)]);
        let mut expected = vec![];
        expected.append(&mut get_voyage(&world, &v2(4, 1), &v2(2, 1)).unwrap());
        expected.append(&mut get_voyage(&world, &v2(4, 1), &v2(2, 3)).unwrap());
        expected.append(&mut get_voyage(&world, &v2(4, 3), &v2(2, 1)).unwrap());
        expected.append(&mut get_voyage(&world, &v2(4, 3), &v2(2, 3)).unwrap());
        let expected = expected.drain(..).collect();
        assert_eq!(actual, expected);
    }
}