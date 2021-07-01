use super::*;

pub trait TravelModeChange {
    fn travel_mode_change(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> bool;
}

impl<T> TravelModeChange for T
where
    T: TravelModeFn,
{
    fn travel_mode_change(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> bool {
        self.travel_mode_here(world, from).map(|mode| mode.class())
            != self.travel_mode_here(world, to).map(|mode| mode.class())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::M;
    use std::collections::HashMap;

    struct MockTravelModeFn {
        map: HashMap<V2<usize>, Option<TravelMode>>,
    }

    impl TravelModeFn for MockTravelModeFn {
        fn travel_mode_between(
            &self,
            _: &World,
            _: &V2<usize>,
            _: &V2<usize>,
        ) -> Option<TravelMode> {
            None
        }

        fn travel_mode_here(&self, _: &World, position: &V2<usize>) -> Option<TravelMode> {
            self.map[position].clone()
        }
    }

    fn world() -> World {
        World::new(M::zeros(3, 3), 0.0)
    }

    fn test_travel_mode_change(from: Option<TravelMode>, to: Option<TravelMode>, expected: bool) {
        let map = hashmap! {
            v2(0, 0) => from,
            v2(1, 1) => to,
        };
        let travel_mode_fn = MockTravelModeFn { map };
        assert_eq!(
            travel_mode_fn.travel_mode_change(&world(), &v2(0, 0), &v2(1, 1)),
            expected
        );
        assert_eq!(
            travel_mode_fn.travel_mode_change(&world(), &v2(1, 1), &v2(0, 0)),
            expected
        );
    }

    #[test]
    fn test_travel_mode_change_land_to_land() {
        test_travel_mode_change(Some(TravelMode::Walk), Some(TravelMode::Walk), false);
    }

    #[test]
    fn test_travel_mode_change_land_to_water() {
        test_travel_mode_change(Some(TravelMode::Walk), Some(TravelMode::Sea), true);
    }

    #[test]
    fn test_travel_mode_change_land_to_empty() {
        test_travel_mode_change(Some(TravelMode::Walk), None, true);
    }

    #[test]
    fn test_travel_mode_change_water_to_water() {
        test_travel_mode_change(Some(TravelMode::Sea), Some(TravelMode::Sea), false);
    }

    #[test]
    fn test_travel_mode_change_water_to_empty() {
        test_travel_mode_change(Some(TravelMode::Sea), None, true);
    }

    #[test]
    fn test_travel_mode_change_empty_to_empty() {
        test_travel_mode_change(None, None, false);
    }
}
