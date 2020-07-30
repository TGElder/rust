use super::*;

pub trait TravelModeChange {
    fn travel_mode_change(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> bool;
    fn check_for_port(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<V2<usize>>;
}

impl<T> TravelModeChange for T
where
    T: TravelModeFn,
{
    fn travel_mode_change(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> bool {
        let from_classes = self.travel_mode_classes_here(world, from);
        let to_classes = self.travel_mode_classes_here(world, to);
        if from_classes.is_empty() && to_classes.is_empty() {
            return false;
        }
        !from_classes.intersection(&to_classes).any(|_| true)
    }

    #[allow(dead_code)] // TODO
    fn check_for_port(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<V2<usize>> {
        let from_classes = self.travel_mode_classes_here(world, from);
        let to_classes = self.travel_mode_classes_here(world, to);
        if from_classes.is_empty() || to_classes.is_empty() {
            return None;
        }
        let from_water = from_classes.contains(&TravelModeClass::Water);
        let to_water = to_classes.contains(&TravelModeClass::Water);
        if from_water && !to_water {
            Some(*to)
        } else if !from_water && to_water {
            Some(*from)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::M;
    use std::collections::HashMap;

    struct MockTravelModeFn {
        map: HashMap<V2<usize>, Vec<TravelMode>>,
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

        fn travel_modes_here(&self, _: &World, position: &V2<usize>) -> Vec<TravelMode> {
            self.map[position].clone()
        }
    }

    fn world() -> World {
        World::new(M::zeros(3, 3), 0.0)
    }

    fn test_travel_mode_change(from: Vec<TravelMode>, to: Vec<TravelMode>, expected: bool) {
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
        test_travel_mode_change(vec![TravelMode::Walk], vec![TravelMode::Walk], false);
    }

    #[test]
    fn test_travel_mode_change_land_to_water() {
        test_travel_mode_change(vec![TravelMode::Walk], vec![TravelMode::Sea], true);
    }

    #[test]
    fn test_travel_mode_change_land_to_mix() {
        test_travel_mode_change(
            vec![TravelMode::Walk],
            vec![TravelMode::Walk, TravelMode::Sea],
            false,
        );
    }

    #[test]
    fn test_travel_mode_change_land_to_empty() {
        test_travel_mode_change(vec![TravelMode::Walk], vec![], true);
    }

    #[test]
    fn test_travel_mode_change_water_to_water() {
        test_travel_mode_change(vec![TravelMode::Sea], vec![TravelMode::Sea], false);
    }

    #[test]
    fn test_travel_mode_change_water_to_mix() {
        test_travel_mode_change(
            vec![TravelMode::Sea],
            vec![TravelMode::Walk, TravelMode::Sea],
            false,
        );
    }

    #[test]
    fn test_travel_mode_change_water_to_empty() {
        test_travel_mode_change(vec![TravelMode::Sea], vec![], true);
    }

    #[test]
    fn test_travel_mode_change_mix_to_mix() {
        test_travel_mode_change(
            vec![TravelMode::Walk, TravelMode::Sea],
            vec![TravelMode::Walk, TravelMode::Sea],
            false,
        );
    }

    #[test]
    fn test_travel_mode_change_mix_to_empty() {
        test_travel_mode_change(vec![TravelMode::Walk, TravelMode::Sea], vec![], true);
    }

    #[test]
    fn test_travel_mode_change_empty_to_empty() {
        test_travel_mode_change(vec![], vec![], false);
    }

    fn test_check_for_port(
        from: Vec<TravelMode>,
        to: Vec<TravelMode>,
        from_port: bool,
        to_port: bool,
    ) {
        let map = hashmap! {
            v2(0, 0) => from,
            v2(1, 1) => to,
        };
        let travel_mode_fn = MockTravelModeFn { map };
        let expected = if from_port {
            Some(v2(0, 0))
        } else if to_port {
            Some(v2(1, 1))
        } else {
            None
        };
        assert_eq!(
            travel_mode_fn.check_for_port(&world(), &v2(0, 0), &v2(1, 1)),
            expected
        );
        assert_eq!(
            travel_mode_fn.check_for_port(&world(), &v2(1, 1), &v2(0, 0)),
            expected
        );
    }

    #[test]
    fn test_check_for_port_land_to_land() {
        test_check_for_port(vec![TravelMode::Walk], vec![TravelMode::Walk], false, false);
    }

    #[test]
    fn test_check_for_port_land_to_water() {
        test_check_for_port(vec![TravelMode::Walk], vec![TravelMode::Sea], true, false);
    }

    #[test]
    fn test_check_for_port_land_to_mix() {
        test_check_for_port(
            vec![TravelMode::Walk],
            vec![TravelMode::Walk, TravelMode::Sea],
            true,
            false,
        );
    }

    #[test]
    fn test_check_for_port_land_to_empty() {
        test_check_for_port(vec![TravelMode::Walk], vec![], false, false);
    }

    #[test]
    fn test_check_for_port_water_to_water() {
        test_check_for_port(vec![TravelMode::Sea], vec![TravelMode::Sea], false, false);
    }

    #[test]
    fn test_check_for_port_water_to_mix() {
        test_check_for_port(
            vec![TravelMode::Sea],
            vec![TravelMode::Walk, TravelMode::Sea],
            false,
            false,
        );
    }

    #[test]
    fn test_check_for_port_water_to_empty() {
        test_check_for_port(vec![TravelMode::Sea], vec![], false, false);
    }

    #[test]
    fn test_check_for_port_mix_to_mix() {
        test_check_for_port(
            vec![TravelMode::Walk, TravelMode::Sea],
            vec![TravelMode::Walk, TravelMode::Sea],
            false,
            false,
        );
    }

    #[test]
    fn test_check_for_port_mix_to_empty() {
        test_check_for_port(
            vec![TravelMode::Walk, TravelMode::Sea],
            vec![],
            false,
            false,
        );
    }

    #[test]
    fn test_check_for_port_empty_to_empty() {
        test_check_for_port(vec![], vec![], false, false);
    }
}
