mod active_process;
mod passive_process;

use commons::async_trait::async_trait;
use commons::fn_sender::{fn_channel, FnMessageExt, FnReceiver, FnSender};
use commons::futures::executor::ThreadPool;
use commons::futures::future::{FutureExt, RemoteHandle};

pub use active_process::*;
pub use passive_process::*;

pub struct Program<T>
where
    T: Send,
{
    actor: T,
    actor_rx: FnReceiver<T>,
    tx: FnSender<Self>,
    rx: FnReceiver<Self>,
    run: bool,
}

impl<T> Program<T>
where
    T: Send,
{
    fn new(actor: T, actor_rx: FnReceiver<T>) -> Program<T> {
        let (tx, rx) = fn_channel();
        Program {
            actor,
            actor_rx,
            tx,
            rx,
            run: true,
        }
    }
}

#[async_trait]
pub trait Process: Send {
    type T: Send + 'static;

    fn state(&self) -> &ProcessState<Self::T>;
    fn mut_state(&mut self) -> &mut ProcessState<Self::T>;
    async fn step(t: &mut Program<Self::T>);

    async fn run(mut t: Program<Self::T>) -> Program<Self::T> {
        while t.run {
            Self::step(&mut t).await;
        }
        t.run = true;
        t
    }

    fn start(&mut self, pool: &ThreadPool) {
        if let ProcessState::Paused(program) = self.mut_state() {
            let program = program.take().unwrap();
            let tx = program.tx.clone();
            let (runnable, handle) = async move { Self::run(program).await }.remote_handle();
            pool.spawn_ok(runnable);
            *self.mut_state() = ProcessState::Running { handle, tx };
        } else {
            panic!("Cannot start program: program is not paused!");
        }
    }

    async fn pause(&mut self) {
        if let ProcessState::Running { handle, tx } = self.mut_state() {
            tx.send(|program| program.run = false).await;
            *self.mut_state() = ProcessState::Paused(Some(handle.await));
        } else {
            panic!("Cannot pause program: program is not running!");
        }
    }
}

pub enum ProcessState<T>
where
    T: Send,
{
    Running {
        handle: RemoteHandle<Program<T>>,
        tx: FnSender<Program<T>>,
    },
    Paused(Option<Program<T>>),
}

pub trait Persistable {
    fn save(&self, path: &str);
    fn load(&mut self, path: &str);
}

impl<T> Persistable for T
where
    T: Process,
    <T as Process>::T: Persistable,
{
    fn save(&self, path: &str) {
        if let ProcessState::Paused(program) = self.state() {
            program.as_ref().unwrap().actor.save(path);
        } else {
            panic!("Cannot save program state: program is not paused!");
        }
    }

    fn load(&mut self, path: &str) {
        if let ProcessState::Paused(program) = self.mut_state() {
            program.as_mut().unwrap().actor.load(path);
        } else {
            panic!("Cannot load program state: program is not paused!");
        }
    }
}
