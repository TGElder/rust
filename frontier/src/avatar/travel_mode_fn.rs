use super::*;

use crate::world::World;
use commons::*;
use std::collections::HashSet;

pub trait TravelModeFn {
    fn travel_mode_between(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<AvatarTravelMode>;

    fn travel_modes_here(&self, world: &World, position: &V2<usize>) -> Vec<AvatarTravelMode>;

    fn travel_mode_classes_here(
        &self,
        world: &World,
        position: &V2<usize>,
    ) -> HashSet<TravelModeClass> {
        self.travel_modes_here(world, position)
            .into_iter()
            .map(|mode| mode.class())
            .collect()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    struct FixedTravelModes {
        modes: Vec<AvatarTravelMode>,
    }

    impl TravelModeFn for FixedTravelModes {
        fn travel_mode_between(
            &self,
            _: &World,
            _: &V2<usize>,
            _: &V2<usize>,
        ) -> Option<AvatarTravelMode> {
            None
        }

        fn travel_modes_here(&self, _: &World, _: &V2<usize>) -> Vec<AvatarTravelMode> {
            self.modes.clone()
        }
    }

    fn world() -> World {
        World::new(M::zeros(3, 3), 0.0)
    }

    fn test_travel_mode_classes_here(
        modes: Vec<AvatarTravelMode>,
        classes: HashSet<TravelModeClass>,
    ) {
        assert_eq!(
            FixedTravelModes { modes }.travel_mode_classes_here(&world(), &v2(0, 0)),
            classes
        );
    }

    #[test]
    fn test_travel_mode_classes_here_walk() {
        test_travel_mode_classes_here(
            vec![AvatarTravelMode::Walk],
            hashset! {TravelModeClass::Land},
        );
    }

    #[test]
    fn test_travel_mode_classes_here_road() {
        test_travel_mode_classes_here(
            vec![AvatarTravelMode::Road],
            hashset! {TravelModeClass::Land},
        );
    }

    #[test]
    fn test_travel_mode_classes_here_planned_road() {
        test_travel_mode_classes_here(
            vec![AvatarTravelMode::PlannedRoad],
            hashset! {TravelModeClass::Land},
        );
    }

    #[test]
    fn test_travel_mode_classes_here_stream() {
        test_travel_mode_classes_here(
            vec![AvatarTravelMode::Stream],
            hashset! {TravelModeClass::Land},
        );
    }

    #[test]
    fn test_travel_mode_classes_here_river() {
        test_travel_mode_classes_here(
            vec![AvatarTravelMode::River],
            hashset! {TravelModeClass::Water},
        );
    }

    #[test]
    fn test_travel_mode_classes_here_sea() {
        test_travel_mode_classes_here(
            vec![AvatarTravelMode::Sea],
            hashset! {TravelModeClass::Water},
        );
    }

    #[test]
    fn test_travel_mode_classes_here_mixed() {
        test_travel_mode_classes_here(
            vec![AvatarTravelMode::Road, AvatarTravelMode::River],
            hashset! {TravelModeClass::Land, TravelModeClass::Water},
        );
    }

    #[test]
    fn test_travel_mode_classes_here_empty() {
        test_travel_mode_classes_here(vec![], hashset! {});
    }
}
