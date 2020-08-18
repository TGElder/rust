use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedRoad {
    pub horizontal: PlannedRoad1D,
    pub vertical: PlannedRoad1D,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlannedRoad1D {
    pub from: Option<u128>,
    pub to: Option<u128>,
}

impl PlannedRoad1D {
    pub fn either(&self) -> bool {
        self.from.is_some() || self.to.is_some()
    }
}

impl PlannedRoad {
    pub fn get(&self, horizontal: bool) -> &PlannedRoad1D {
        if horizontal {
            &self.horizontal
        } else {
            &self.vertical
        }
    }

    pub fn get_mut(&mut self, horizontal: bool) -> &mut PlannedRoad1D {
        if horizontal {
            &mut self.horizontal
        } else {
            &mut self.vertical
        }
    }

    pub fn here(&self) -> bool {
        self.horizontal.either() || self.vertical.either()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        let planned_road = PlannedRoad {
            horizontal: PlannedRoad1D {
                from: Some(1),
                to: Some(2),
            },
            vertical: PlannedRoad1D {
                from: Some(3),
                to: Some(4),
            },
        };

        assert_eq!(
            planned_road.get(true),
            &PlannedRoad1D {
                from: Some(1),
                to: Some(2),
            }
        );
        assert_eq!(
            planned_road.get(false),
            &PlannedRoad1D {
                from: Some(3),
                to: Some(4),
            }
        );
    }

    #[test]
    fn get_mut() {
        let mut planned_road = PlannedRoad {
            horizontal: PlannedRoad1D {
                from: Some(1),
                to: Some(2),
            },
            vertical: PlannedRoad1D {
                from: Some(3),
                to: Some(4),
            },
        };

        assert_eq!(
            planned_road.get_mut(true),
            &mut PlannedRoad1D {
                from: Some(1),
                to: Some(2),
            }
        );
        assert_eq!(
            planned_road.get_mut(false),
            &mut PlannedRoad1D {
                from: Some(3),
                to: Some(4),
            }
        );
    }

    #[test]
    fn either() {
        assert!(!PlannedRoad1D {
            from: None,
            to: None
        }
        .either());
        assert!(PlannedRoad1D {
            from: Some(1),
            to: None
        }
        .either());
        assert!(PlannedRoad1D {
            from: None,
            to: Some(1)
        }
        .either());
        assert!(PlannedRoad1D {
            from: Some(1),
            to: Some(1)
        }
        .either());
    }

    #[test]
    fn some() {
        assert!(!PlannedRoad {
            horizontal: PlannedRoad1D {
                from: None,
                to: None,
            },
            vertical: PlannedRoad1D {
                from: None,
                to: None
            },
        }
        .here());
        assert!(PlannedRoad {
            horizontal: PlannedRoad1D {
                from: Some(1),
                to: None,
            },
            vertical: PlannedRoad1D {
                from: None,
                to: None
            },
        }
        .here());
        assert!(PlannedRoad {
            horizontal: PlannedRoad1D {
                from: None,
                to: None,
            },
            vertical: PlannedRoad1D {
                from: Some(1),
                to: None
            },
        }
        .here());
        assert!(PlannedRoad {
            horizontal: PlannedRoad1D {
                from: Some(1),
                to: None,
            },
            vertical: PlannedRoad1D {
                from: Some(1),
                to: None
            },
        }
        .here());
    }
}
