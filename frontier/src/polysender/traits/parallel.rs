use commons::futures::Future;

use crate::polysender::Polysender;

pub trait Parallel {
    fn parallel<FUT>(&self, future: FUT)
    where
        FUT: Future<Output = ()> + Send + 'static;
}

impl Parallel for Polysender {
    fn parallel<FUT>(&self, future: FUT)
    where
        FUT: Future<Output = ()> + Send + 'static,
    {
        self.thread_pool.spawn_ok(future);
    }
}
