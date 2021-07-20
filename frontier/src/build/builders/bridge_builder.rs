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
    use crate::bridge::{Bridge, BridgeType, Pier, Segment};

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
    fn can_build_bridge() {
        // Given
        let game = Arc::new(Mutex::new(hashset! {}));
        let builder = BridgeBuilder::new(game);
        let bridge = Bridge {
            segments: vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_millis(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 3.0,
                        platform: true,
                    },
                    duration: Duration::from_millis(2),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

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
        let bridge = Bridge {
            segments: vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_millis(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 3.0,
                        platform: true,
                    },
                    duration: Duration::from_millis(2),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

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
        let bridge_1 = Bridge {
            segments: vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_millis(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 3.0,
                        platform: true,
                    },
                    duration: Duration::from_millis(2),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };
        let bridge_2 = Bridge {
            segments: vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(0, 1),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_millis(1),
                },
                Segment {
                    from: Pier {
                        position: v2(0, 1),
                        elevation: 2.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(0, 2),
                        elevation: 3.0,
                        platform: true,
                    },
                    duration: Duration::from_millis(2),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

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
