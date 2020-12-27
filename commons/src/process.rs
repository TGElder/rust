use async_channel::{unbounded, Sender};
use async_trait::async_trait;
use crate::fn_sender::{FnMessageExt, FnReceiver};
use log::error;
use futures::executor::ThreadPool;
use futures::future::{FutureExt, RemoteHandle};

use std::any::type_name;

pub struct Process<T> {
    state: Option<ProcessState<T>>,
}

enum ProcessState<T> {
    Paused {
        actor: T,
        actor_rx: FnReceiver<T>,
    },
    Running {
        shutdown_tx: Sender<()>,
        handle: RemoteHandle<(T, FnReceiver<T>)>,
    },
    Draining {
        actor: T,
        shutdown_tx: Sender<()>,
        handle: RemoteHandle<FnReceiver<T>>,
    },
}

impl<T> Process<T>
where
    T: Send + 'static,
{
    pub fn new(actor: T, actor_rx: FnReceiver<T>) -> Process<T> {
        Process {
            state: Some(ProcessState::Paused { actor, actor_rx }),
        }
    }

    async fn actor_and_rx(&mut self) -> (T, FnReceiver<T>) {
        match self.state.take().unwrap() {
            ProcessState::Paused { actor, actor_rx } => (actor, actor_rx),
            ProcessState::Running {
                shutdown_tx,
                handle,
            } => {
                shutdown_tx.send(()).await.unwrap();
                handle.await
            }
            ProcessState::Draining {
                actor,
                shutdown_tx,
                handle,
            } => {
                shutdown_tx.send(()).await.unwrap();
                (actor, handle.await)
            }
        }
    }

    pub async fn run_passive(&mut self, pool: &ThreadPool) {
        let (mut actor, mut actor_rx) = self.actor_and_rx().await;
        let (shutdown_tx, shutdown_rx) = unbounded();
        let (runnable, handle) = async move {
            loop {
                select! {
                    mut message = shutdown_rx.recv().fuse() => {
                        actor_rx.get_messages().apply(&mut actor).await;
                        return (actor, actor_rx)
                    },
                    mut message = actor_rx.get_message().fuse() => message.apply(&mut actor).await,
                }
            }
        }
        .remote_handle();
        pool.spawn_ok(runnable);
        self.state = Some(ProcessState::Running {
            shutdown_tx,
            handle,
        });
    }

    pub async fn drain(&mut self, pool: &ThreadPool) {
        let (actor, mut actor_rx) = self.actor_and_rx().await;
        let (shutdown_tx, shutdown_rx) = unbounded();
        let (runnable, handle) = async move {
            let mut count = 0;
            loop {
                select! {
                    mut message = shutdown_rx.recv().fuse() => {
                        count += actor_rx.get_messages().len();
                        if count > 0 {
                            error!(
                                "{} messages for {:?} were drained!",
                                count,
                                type_name::<T>()
                            );
                        }
                        return actor_rx
                    },
                    mut message = actor_rx.get_message().fuse() => count += 1,
                }
            }
        }
        .remote_handle();
        pool.spawn_ok(runnable);
        self.state = Some(ProcessState::Draining {
            actor,
            shutdown_tx,
            handle,
        });
    }
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
        let (mut actor, mut actor_rx) = self.actor_and_rx().await;
        let (shutdown_tx, shutdown_rx) = unbounded();
        let (runnable, handle) = async move {
            loop {
                actor_rx.get_messages().apply(&mut actor).await;
                if let Ok(()) = shutdown_rx.try_recv() {
                    return (actor, actor_rx);
                }
                actor.step().await;
            }
        }
        .remote_handle();
        pool.spawn_ok(runnable);
        self.state = Some(ProcessState::Running {
            shutdown_tx,
            handle,
        })
    }
}

pub trait Persistable {
    fn save(&self, path: &str);
    fn load(&mut self, path: &str);
}

impl<T> Process<T>
where
    T: Persistable,
{
    pub fn save(&self, path: &str) {
        let actor = match self.state.as_ref() {
            Some(ProcessState::Paused { actor, .. }) => actor,
            Some(ProcessState::Draining { actor, .. }) => actor,
            _ => panic!("Can only save paused or draining process!"),
        };
        actor.save(path);
    }

    pub fn load(&mut self, path: &str) {
        let actor = match self.state.as_mut() {
            Some(ProcessState::Paused { actor, .. }) => actor,
            Some(ProcessState::Draining { actor, .. }) => actor,
            _ => panic!("Can only load to paused or draining process!"),
        };
        actor.load(path);
    }
}
