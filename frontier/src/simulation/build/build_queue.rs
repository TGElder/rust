use super::*;

use std::collections::hash_map::Entry;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct BuildQueue {
    queue: HashMap<BuildKey, BuildInstruction>,
}

impl BuildQueue {
    pub fn insert(&mut self, build_instruction: BuildInstruction) {
        let key = build_instruction.what.key();
        match self.queue.entry(key) {
            Entry::Occupied(mut value) if build_instruction.when < value.get().when => {
                value.insert(build_instruction);
            }
            Entry::Vacant(cell) => {
                cell.insert(build_instruction);
            }
            _ => (),
        };
    }

    pub fn remove(&mut self, build_key: &BuildKey) {
        self.queue.remove(build_key);
    }

    pub fn take_instructions_before(&mut self, micros: u128) -> Vec<BuildInstruction> {
        let (to_build, to_retain) = self
            .queue
            .drain()
            .partition(|(_, BuildInstruction { when, .. })| *when <= micros);
        self.queue = to_retain;
        to_build
            .into_iter()
            .map(|(_, instruction)| instruction)
            .collect()
    }

    pub fn get(&self, build_key: &BuildKey) -> Option<&BuildInstruction> {
        self.queue.get(build_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::edge::Edge;
    use commons::v2;

    #[test]
    fn insert_should_remove_later_instruction_with_same_key() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut build_queue = BuildQueue::default();
        let earlier = BuildInstruction {
            what: Build::Road(edge),
            when: 0,
        };
        let later = BuildInstruction {
            what: Build::Road(edge),
            when: 100,
        };

        build_queue.insert(later);

        // When
        build_queue.insert(earlier.clone());

        // Then
        assert_eq!(
            build_queue.queue,
            hashmap! {
                Build::Road(edge).key() => earlier
            }
        );
    }

    #[test]
    fn insert_should_not_remove_earlier_instruction_with_same_key() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut build_queue = BuildQueue::default();
        let earlier = BuildInstruction {
            what: Build::Road(edge),
            when: 0,
        };
        let later = BuildInstruction {
            what: Build::Road(edge),
            when: 100,
        };

        build_queue.insert(earlier.clone());

        // When
        build_queue.insert(later);

        // Then
        assert_eq!(
            build_queue.queue,
            hashmap! {
                Build::Road(edge).key() => earlier
            }
        );
    }

    #[test]
    fn remove_should_always_remove_instruction() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut build_queue = BuildQueue::default();
        let instruction = BuildInstruction {
            what: Build::Road(edge),
            when: 0,
        };

        build_queue.insert(instruction);

        // When
        build_queue.remove(&BuildKey::Road(edge));

        // Then
        assert_eq!(build_queue.queue, hashmap! {});
    }

    #[test]
    fn take_instructions_before() {
        // Given
        let mut build_queue = BuildQueue::default();
        let before_edge = Edge::new(v2(1, 2), v2(1, 3));
        let before = BuildInstruction {
            what: Build::Road(before_edge),
            when: 0,
        };
        let after_edge = Edge::new(v2(2, 3), v2(2, 4));
        let after = BuildInstruction {
            what: Build::Road(after_edge),
            when: 100,
        };
        build_queue.insert(before.clone());
        build_queue.insert(after.clone());

        // When
        let actual = build_queue.take_instructions_before(50);

        // Then
        assert_eq!(actual, vec![before]);
        assert_eq!(
            build_queue.queue,
            hashmap! {
                Build::Road(after_edge).key() => after
            }
        );
    }

    #[test]
    fn take_instructions_before_none_before() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut build_queue = BuildQueue::default();
        let after = BuildInstruction {
            what: Build::Road(edge),
            when: 100,
        };
        build_queue.insert(after.clone());

        // When
        let actual = build_queue.take_instructions_before(50);

        // Then
        assert_eq!(actual, vec![]);
        assert_eq!(
            build_queue.queue,
            hashmap! {
                Build::Road(edge).key() => after
            }
        );
    }

    #[test]
    fn take_instructions_before_none_after() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut build_queue = BuildQueue::default();
        let before = BuildInstruction {
            what: Build::Road(edge),
            when: 0,
        };
        build_queue.insert(before.clone());

        // When
        let actual = build_queue.take_instructions_before(50);

        // Then
        assert_eq!(actual, vec![before]);
        assert_eq!(build_queue.queue, hashmap! {});
    }
}
