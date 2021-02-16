use futures::Future;

pub trait RunInBackground {
    fn run_in_background<Fut>(&self, future: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static;
}
