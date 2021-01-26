use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::time::Instant;

pub struct Clock<T>
where
    T: Now,
{
    baseline_instant: Instant,
    now: T,
    default_speed: f32,
    state: ClockState,
}

pub trait Now {
    fn instant(&mut self) -> Instant;
}

#[derive(Serialize, Deserialize)]
struct ClockState {
    baseline_micros: u128,
    speed: f32,
}

impl<T> Clock<T>
where
    T: Now,
{
    pub fn new(mut now: T, default_speed: f32) -> Clock<T> {
        Clock {
            baseline_instant: now.instant(),
            now,
            default_speed,
            state: ClockState {
                baseline_micros: 0,
                speed: default_speed,
            },
        }
    }

    pub fn init(&mut self) {
        self.baseline_instant = self.now.instant();
    }

    pub fn get_micros(&mut self) -> u128 {
        let instant = &self.now.instant();
        self.get_micros_at(instant)
    }

    fn get_micros_at(&self, instant: &Instant) -> u128 {
        self.state.baseline_micros
            + (instant.duration_since(self.baseline_instant).as_micros() as f32 * self.state.speed)
                .round() as u128
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.update_baseline();
        self.state.speed = speed;
    }

    pub fn adjust_speed(&mut self, factor: f32) {
        self.set_speed(self.state.speed * factor);
    }

    pub fn pause(&mut self) {
        self.set_speed(0.0);
    }

    pub fn resume(&mut self) {
        self.set_speed(self.default_speed)
    }

    fn update_baseline(&mut self) {
        let new_baseline_instant = self.now.instant();
        self.state.baseline_micros = self.get_micros_at(&new_baseline_instant);
        self.baseline_instant = new_baseline_instant;
    }

    pub fn save(&mut self, path: &str) {
        self.update_baseline();
        let path = get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
        self.baseline_instant = self.now.instant();
    }
}

fn get_path(path: &str) -> String {
    format!("{}.clock", path)
}

pub struct RealTime {}

impl Now for RealTime {
    fn instant(&mut self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use commons::Arm;

    use super::*;

    struct MockNow {
        baseline_instant: Instant,
        offset_micros: u64,
    }

    impl Default for MockNow {
        fn default() -> Self {
            MockNow {
                baseline_instant: Instant::now(),
                offset_micros: 0,
            }
        }
    }

    impl Now for Arm<MockNow> {
        fn instant(&mut self) -> Instant {
            let mock_now = self.lock().unwrap();
            mock_now.baseline_instant + Duration::from_micros(mock_now.offset_micros)
        }
    }

    #[test]
    fn test_get_micros() {
        // Given
        let now = Arc::new(Mutex::new(MockNow::default()));
        let mut clock = Clock::new(now.clone(), 2.0);

        // When
        now.lock().unwrap().offset_micros = 1;

        // Then
        assert_eq!(clock.get_micros(), 2);
    }

    #[test]
    fn test_init() {
        // Given
        let now = Arc::new(Mutex::new(MockNow::default()));
        let mut clock = Clock::new(now.clone(), 2.0);

        // When
        now.lock().unwrap().offset_micros = 1;
        clock.init();

        // Then
        assert_eq!(clock.get_micros(), 0);
    }

    #[test]
    fn test_set_speed() {
        // Given
        let now = Arc::new(Mutex::new(MockNow::default()));
        let mut clock = Clock::new(now.clone(), 2.0);

        // When
        clock.set_speed(4.0);
        now.lock().unwrap().offset_micros = 1;

        // Then
        assert_eq!(clock.get_micros(), 4);
    }

    #[test]
    fn test_adjust_speed() {
        // Given
        let now = Arc::new(Mutex::new(MockNow::default()));
        let mut clock = Clock::new(now.clone(), 2.0);

        // When
        clock.adjust_speed(0.5);
        now.lock().unwrap().offset_micros = 1;

        // Then
        assert_eq!(clock.get_micros(), 1);
    }

    #[test]
    fn test_pause() {
        // Given
        let now = Arc::new(Mutex::new(MockNow::default()));
        let mut clock = Clock::new(now.clone(), 2.0);

        // When
        clock.pause();
        now.lock().unwrap().offset_micros = 1;

        // Then
        assert_eq!(clock.get_micros(), 0);
    }

    #[test]
    fn test_resume() {
        // Given
        let now = Arc::new(Mutex::new(MockNow::default()));
        let mut clock = Clock::new(now.clone(), 2.0);
        clock.pause();

        // When
        now.lock().unwrap().offset_micros = 1;
        clock.resume();
        now.lock().unwrap().offset_micros = 2;

        // Then
        assert_eq!(clock.get_micros(), 2);
    }

    #[test]
    fn test_save_load() {
        // Given
        let now = Arc::new(Mutex::new(MockNow::default()));
        let mut clock = Clock::new(now.clone(), 2.0);
        clock.save("test_save.clock");

        // When
        now.lock().unwrap().offset_micros = 10;
        clock.load("test_save.clock");

        // Then
        assert_eq!(clock.get_micros(), 0);
    }
}
