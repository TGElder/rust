use futures::Future;

pub trait Background{
    fn background<Fut>(&self, future: Fut)
    where
            Fut: Future<Output = ()> + Send + 'static;
}