use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use commons::log::debug;
use futures::executor::ThreadPool;
use futures::Future;

pub struct BackgroundService {
    pool: ThreadPool,
    tasks: Arc<AtomicUsize>,
}

impl BackgroundService {
    pub fn new(pool: ThreadPool) -> Self {
        BackgroundService {
            pool,
            tasks: Arc::default(),
        }
    }

    pub fn run_in_background<Fut>(&self, future: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        let tasks = self.tasks.clone();
        tasks.fetch_add(1, Ordering::Relaxed);
        self.pool.spawn_ok(async move {
            future.await;
            tasks.fetch_sub(1, Ordering::Relaxed);
        });
    }

    pub fn wait_on_tasks(&self) {
        while !self.zero_tasks() {}
    }

    fn zero_tasks(&self) -> bool {
        let task_count = self.tasks.load(Ordering::Relaxed);
        debug!("{} running tasks", task_count);
        task_count == 0
    }
}
