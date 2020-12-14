use commons::async_trait::async_trait;
use commons::fn_sender::{fn_channel, FnMessageExt, FnReceiver, FnSender};
use commons::futures::executor::ThreadPool;
use commons::futures::future::{FutureExt, RemoteHandle};

use crate::system::Persistable;

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
    pub fn new(actor: T, actor_rx: FnReceiver<T>) -> Self {
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
            panic!("Cannot run program: program is not paused!");
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

pub struct PassiveProcess<T>
where
    T: Send + 'static,
{
    state: ProcessState<T>,
}

impl<T> PassiveProcess<T>
where
    T: Send + 'static,
{
    pub fn new(program: Program<T>) -> PassiveProcess<T> {
        PassiveProcess {
            state: ProcessState::Paused(Some(program)),
        }
    }
}

#[async_trait]
impl<X> Process for PassiveProcess<X>
where
    X: Send + 'static,
{
    type T = X;

    fn state(&self) -> &ProcessState<Self::T> {
        &self.state
    }

    fn mut_state(&mut self) -> &mut ProcessState<Self::T> {
        &mut self.state
    }

    async fn step(t: &mut Program<Self::T>) {
        select! {
            mut message = t.rx.get_message().fuse() => message.apply(t).await,
            mut message = t.actor_rx.get_message().fuse() => message.apply(&mut t.actor).await,
        }
    }
}

pub struct ActiveProcess<T>
where
    T: Step + Send + 'static,
{
    state: ProcessState<T>,
}

impl<T> ActiveProcess<T>
where
    T: Step + Send + 'static,
{
    pub fn new(program: Program<T>) -> ActiveProcess<T> {
        ActiveProcess {
            state: ProcessState::Paused(Some(program)),
        }
    }
}

#[async_trait]
pub trait Step {
    async fn step(&mut self);
}

#[async_trait]
impl<X> Process for ActiveProcess<X>
where
    X: Step + Send + 'static,
{
    type T = X;

    fn state(&self) -> &ProcessState<Self::T> {
        &self.state
    }

    fn mut_state(&mut self) -> &mut ProcessState<Self::T> {
        &mut self.state
    }

    async fn step(t: &mut Program<Self::T>) {
        t.rx.get_messages().apply(t).await;
        if !t.run {
            return;
        }
        t.actor_rx.get_messages().apply(&mut t.actor).await;
        t.actor.step().await;
    }
}

impl<T> Program<T>
where
    T: Send + Persistable,
{
    pub fn save(&self, path: &str) {
        self.actor.save(path);
    }

    pub fn load(&mut self, path: &str) {
        self.actor.load(path);
    }
}

impl<T> Persistable for T
where
    T: Process,
    <T as Process>::T: Persistable,
{
    fn save(&self, path: &str) {
        if let ProcessState::Paused(program) = self.state() {
            program.as_ref().unwrap().save(path);
        } else {
            panic!("Cannot save program state: program is not paused!");
        }
    }

    fn load(&mut self, path: &str) {
        if let ProcessState::Paused(program) = self.mut_state() {
            program.as_mut().unwrap().load(path);
        } else {
            panic!("Cannot load program state: program is not paused!");
        }
    }
}
