use crate::fn_sender::{FnMessageExt, FnReceiver};
use async_channel::{unbounded, Receiver, Sender};
use async_trait::async_trait;
use futures::executor::ThreadPool;
use futures::future::{FutureExt, RemoteHandle};
use log::{debug, error};

use std::any::type_name;

pub struct Process<T> {
    state: Option<ProcessState<T>>,
}

enum ProcessState<T> {
    Paused {
        object: T,
        object_rx: FnReceiver<T>,
    },
    Running {
        shutdown_tx: Sender<()>,
        handle: RemoteHandle<(T, FnReceiver<T>)>,
    },
    Draining {
        object: T,
        shutdown_tx: Sender<()>,
        handle: RemoteHandle<FnReceiver<T>>,
    },
}

impl<T> Process<T>
where
    T: Send + 'static,
{
    pub fn new(object: T, object_rx: FnReceiver<T>) -> Process<T> {
        Process {
            state: Some(ProcessState::Paused { object, object_rx }),
        }
    }

    async fn object_and_rx(&mut self) -> (T, FnReceiver<T>) {
        match self.state.take().unwrap() {
            ProcessState::Paused { object, object_rx } => (object, object_rx),
            ProcessState::Running {
                shutdown_tx,
                handle,
            } => {
                shutdown_tx.send(()).await.unwrap();
                handle.await
            }
            ProcessState::Draining {
                object,
                shutdown_tx,
                handle,
            } => {
                shutdown_tx.send(()).await.unwrap();
                (object, handle.await)
            }
        }
    }

    pub async fn run_passive(&mut self, pool: &ThreadPool) {
        debug!("Running {} (passive)", type_name::<T>());
        let (mut object, mut object_rx) = self.object_and_rx().await;
        process_messages(&mut object, &mut object_rx).await;
        let (shutdown_tx, shutdown_rx) = unbounded();
        let handle = run_passive(object, object_rx, shutdown_rx, pool);
        self.state = Some(ProcessState::Running {
            shutdown_tx,
            handle,
        });
    }

    pub async fn drain(&mut self, pool: &ThreadPool, error_on_drain: bool) {
        debug!("Draining {}", type_name::<T>());
        let (object, object_rx) = self.object_and_rx().await;
        let (shutdown_tx, shutdown_rx) = unbounded();
        let handle = drain(object_rx, shutdown_rx, pool, error_on_drain);
        self.state = Some(ProcessState::Draining {
            object,
            shutdown_tx,
            handle,
        });
    }

    pub fn object_ref(&self) -> Result<&T, &'static str> {
        match self.state.as_ref() {
            Some(ProcessState::Paused { object, .. }) => Ok(object),
            Some(ProcessState::Draining { object, .. }) => Ok(object),
            _ => Err("Can only access object in paused or draining process!"),
        }
    }

    pub fn object_mut(&mut self) -> Result<&mut T, &'static str> {
        match self.state.as_mut() {
            Some(ProcessState::Paused { object, .. }) => Ok(object),
            Some(ProcessState::Draining { object, .. }) => Ok(object),
            _ => Err("Can only access object in paused or draining process!"),
        }
    }
}

async fn process_messages<T>(object: &mut T, object_rx: &mut FnReceiver<T>)
where
    T: Send,
{
    let mut messages = object_rx.get_messages();
    if !messages.is_empty() {
        debug!(
            "Processed {} messages for {}",
            messages.len(),
            type_name::<T>()
        );
        messages.apply(object).await;
    }
}

pub fn run_passive<T>(
    mut object: T,
    mut object_rx: FnReceiver<T>,
    shutdown_rx: Receiver<()>,
    pool: &ThreadPool,
) -> RemoteHandle<(T, FnReceiver<T>)>
where
    T: Send + 'static,
{
    let (runnable, handle) = async move {
        loop {
            select! {
                mut message = shutdown_rx.recv().fuse() => {
                    object_rx.get_messages().apply(&mut object).await;
                    return (object, object_rx);
                },
                mut message = object_rx.get_message().fuse() => message.apply(&mut object).await,
            }
        }
    }
    .remote_handle();
    pool.spawn_ok(runnable);
    handle
}

pub fn drain<T>(
    mut object_rx: FnReceiver<T>,
    shutdown_rx: Receiver<()>,
    pool: &ThreadPool,
    error_on_drain: bool,
) -> RemoteHandle<FnReceiver<T>>
where
    T: Send + 'static,
{
    let (runnable, handle) = async move {
        let mut count = 0;
        loop {
            select! {
                mut message = shutdown_rx.recv().fuse() => {
                    count += object_rx.get_messages().len();
                    if error_on_drain && count > 0 {
                        error!(
                            "{} messages for {:?} were drained!",
                            count,
                            type_name::<T>()
                        );
                    }
                    return object_rx
                },
                mut message = object_rx.get_message().fuse() => count += 1,
            }
        }
    }
    .remote_handle();
    pool.spawn_ok(runnable);
    handle
}

#[async_trait]
pub trait Step {
    async fn step(&mut self);
}

impl<T> Process<T>
where
    T: Step + Send + 'static,
{
    pub async fn run_active(&mut self, pool: &ThreadPool) {
        debug!("Running {} (active)", type_name::<T>());
        let (mut object, mut object_rx) = self.object_and_rx().await;
        process_messages(&mut object, &mut object_rx).await;
        let (shutdown_tx, shutdown_rx) = unbounded();
        let handle = run_active(object, object_rx, shutdown_rx, pool);
        self.state = Some(ProcessState::Running {
            shutdown_tx,
            handle,
        })
    }
}

pub fn run_active<T>(
    mut object: T,
    mut object_rx: FnReceiver<T>,
    shutdown_rx: Receiver<()>,
    pool: &ThreadPool,
) -> RemoteHandle<(T, FnReceiver<T>)>
where
    T: Step + Send + 'static,
{
    let (runnable, handle) = async move {
        loop {
            object_rx.get_messages().apply(&mut object).await;
            if let Ok(()) = shutdown_rx.try_recv() {
                return (object, object_rx);
            }
            object.step().await;
        }
    }
    .remote_handle();
    pool.spawn_ok(runnable);
    handle
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Instant;

    use futures::executor::block_on;
    use maplit::hashset;

    use crate::fn_sender::fn_channel;

    use super::*;

    #[derive(Default)]
    struct Object {
        words: HashSet<&'static str>,
    }

    impl Object {
        pub fn say(&mut self, word: &'static str) {
            self.words.insert(word);
        }
    }

    #[async_trait]
    impl Step for Object {
        async fn step(&mut self) {
            self.words.insert("step");
        }
    }

    #[test]
    fn run_passive() {
        // Given
        let object = Object::default();
        let (object_tx, object_rx) = fn_channel();
        let mut process = Process::new(object, object_rx);

        // When
        object_tx.send(move |object| object.say("before"));
        block_on(process.run_passive(&ThreadPool::new().unwrap()));
        block_on(object_tx.send(move |object| object.say("after")));

        // Then
        let (object, _) = block_on(process.object_and_rx());
        assert_eq!(object.words, hashset! {"before", "after"});
    }

    #[test]
    fn run_active() {
        // Given
        let object = Object::default();
        let (object_tx, object_rx) = fn_channel();
        let mut process = Process::new(object, object_rx);

        // When
        object_tx.send(move |object| object.say("before"));
        block_on(process.run_active(&ThreadPool::new().unwrap()));
        block_on(object_tx.send(move |object| object.say("after")));

        // Then
        let start = Instant::now();
        while !block_on(object_tx.send(|object| object.words.contains("step"))) {
            if start.elapsed().as_secs() >= 1 {
                panic!("Object did not step after 1 second!");
            }
        }
        let (object, _) = block_on(process.object_and_rx());
        assert_eq!(object.words, hashset! {"before", "after", "step"});
    }

    #[test]
    fn drain() {
        // Given
        let object = Object::default();
        let (object_tx, object_rx) = fn_channel();
        let mut process = Process::new(object, object_rx);

        // When
        object_tx.send(move |object| object.say("drain"));
        block_on(process.drain(&ThreadPool::new().unwrap(), true));

        // Then
        let (_, mut object_rx) = block_on(process.object_and_rx());
        assert_eq!(object_rx.get_messages().len(), 0);
    }

    #[test]
    fn run_then_drain_then_run() {
        // Given
        let object = Object::default();
        let (object_tx, object_rx) = fn_channel();
        let mut process = Process::new(object, object_rx);

        // When
        let pool = ThreadPool::new().unwrap();

        block_on(process.run_passive(&pool));
        block_on(object_tx.send(move |object| object.say("a")));

        block_on(process.drain(&pool, false));
        object_tx.send(move |object| object.say("b"));

        block_on(process.run_passive(&pool));
        block_on(object_tx.send(move |object| object.say("c")));

        // Then
        let (object, _) = block_on(process.object_and_rx());
        assert_eq!(object.words, hashset! {"a", "c"});
    }

    #[test]
    fn object_ref_and_mut() {
        // Given
        let (_, object_rx) = fn_channel();
        let object = Object::default();
        let mut process = Process::new(object, object_rx);

        // When
        process.object_mut().unwrap().words = hashset! {"ref"};

        // Then
        assert_eq!(process.object_ref().unwrap().words, hashset! {"ref"});
    }
}
