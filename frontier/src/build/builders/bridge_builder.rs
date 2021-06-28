use crate::traits::AddBridge;

use super::*;

pub struct BridgeBuilder<T>
where
    T: AddBridge + Send + Sync,
{
    cx: T,
}

#[async_trait]
impl<T> Builder for BridgeBuilder<T>
where
    T: AddBridge + Send + Sync,
{
    fn can_build(&self, build: &Build) -> bool {
        matches!(build, Build::Bridge(..))
    }

    async fn build(&mut self, build: Vec<Build>) {
        for build in build {
            self.try_build(build).await;
        }
    }
}

impl<T> BridgeBuilder<T>
where
    T: AddBridge + Send + Sync,
{
    pub fn new(cx: T) -> BridgeBuilder<T> {
        BridgeBuilder { cx }
    }

    async fn try_build(&self, build: Build) {
        if let Build::Bridge(bridge) = build {
            self.cx.add_bridge(bridge).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::avatar::Vehicle;
    use crate::bridge::{Bridge, BridgeType};
    use crate::travel_duration::EdgeDuration;

    use super::*;

    use commons::{v2, Arm};
    use futures::executor::block_on;

    #[async_trait]
    impl AddBridge for Arm<HashSet<Bridge>> {
        async fn add_bridge(&self, bridge: Bridge) {
            self.lock().unwrap().insert(bridge);
        }
    }

    #[test]
    fn can_build_road() {
        // Given
        let game = Arc::new(Mutex::new(hashset! {}));
        let builder = BridgeBuilder::new(game);
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_millis(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_millis(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        // When
        let can_build = builder.can_build(&Build::Bridge(bridge));

        // Then
        assert!(can_build);
    }

    #[test]
    fn should_build_bridge() {
        // Given
        let game = Arc::new(Mutex::new(hashset! {}));
        let mut builder = BridgeBuilder::new(game);
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_millis(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_millis(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        // When
        block_on(builder.build(vec![Build::Bridge(bridge.clone())]));

        // Then
        assert_eq!(*builder.cx.lock().unwrap(), hashset! {bridge});
    }

    #[test]
    fn should_build_all_bridges() {
        // Given
        let game = Arc::new(Mutex::new(hashset! {}));
        let mut builder = BridgeBuilder::new(game);
        let bridge_1 = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_millis(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_millis(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();
        let bridge_2 = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(0, 1),
                    duration: Some(Duration::from_millis(1)),
                },
                EdgeDuration {
                    from: v2(0, 1),
                    to: v2(0, 2),
                    duration: Some(Duration::from_millis(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        // When
        block_on(builder.build(vec![
            Build::Bridge(bridge_1.clone()),
            Build::Bridge(bridge_2.clone()),
        ]));

        // Then
        assert_eq!(
            *builder.cx.lock().unwrap(),
            hashset! {
                bridge_1,
                bridge_2
            }
        );
    }
}
