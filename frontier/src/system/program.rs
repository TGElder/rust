use commons::async_trait::async_trait;
use commons::fn_sender::{fn_channel, FnMessageExt, FnReceiver, FnSender};
use commons::futures::future::FutureExt;

use crate::system::Persistable;

#[async_trait]
pub trait Programish {
    type T: Shutdown + Send;

    fn tx(&self) -> &FnSender<Self::T>;
    async fn run(mut self) -> Self;
}

pub trait Shutdown {
    fn shutdown(&mut self);
}

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

    pub fn tx(&self) -> &FnSender<Self> {
        &self.tx
    }

    async fn step(&mut self)
    where
        T: Send,
    {
        select! {
            mut message = self.rx.get_message().fuse() => message.apply(self).await,
            mut message = self.actor_rx.get_message().fuse() => message.apply(&mut self.actor).await,
        }
    }
}

#[async_trait]
impl<T> Programish for Program<T>
where
    T: Send,
{
    type T = Self;

    fn tx(&self) -> &FnSender<Self::T> {
        &self.tx()
    }

    async fn run(mut self) -> Self {
        while self.run {
            self.step().await;
        }
        self.run = true;
        self
    }
}

impl<T> Shutdown for Program<T>
where
    T: Send,
{
    fn shutdown(&mut self) {
        self.run = false;
    }
}

impl<T> Persistable for Program<T>
where
    T: Send + Persistable,
{
    fn save(&self, path: &str) {
        self.actor.save(path);
    }

    fn load(&mut self, path: &str) {
        self.actor.load(path);
    }
}

#[async_trait]
pub trait Step {
    async fn step(&mut self);
}

pub struct BusyProgram<T>
where
    T: Send + Step,
{
    actor: T,
    actor_rx: FnReceiver<T>,
    tx: FnSender<Self>,
    rx: FnReceiver<Self>,
    run: bool,
}

impl<T> BusyProgram<T>
where
    T: Send + Step,
{
    pub fn new(actor: T, actor_rx: FnReceiver<T>) -> Self {
        let (tx, rx) = fn_channel();
        BusyProgram {
            actor,
            actor_rx,
            tx,
            rx,
            run: true,
        }
    }

    pub fn tx(&self) -> &FnSender<Self> {
        &self.tx
    }

    async fn step(&mut self)
    where
        T: Send,
    {
        self.rx.get_messages().apply(self).await;
        if !self.run {
            return;
        }
        self.actor_rx.get_messages().apply(&mut self.actor).await;
        self.actor.step().await;
    }
}

#[async_trait]
impl<T> Programish for BusyProgram<T>
where
    T: Send + Step,
{
    type T = Self;

    fn tx(&self) -> &FnSender<Self::T> {
        &self.tx()
    }

    async fn run(mut self) -> Self {
        while self.run {
            self.step().await;
        }
        self.run = true;
        self
    }
}

impl<T> Shutdown for BusyProgram<T>
where
    T: Send + Step,
{
    fn shutdown(&mut self) {
        self.run = false;
    }
}

impl<T> Persistable for BusyProgram<T>
where
    T: Send + Persistable + Step,
{
    fn save(&self, path: &str) {
        self.actor.save(path);
    }

    fn load(&mut self, path: &str) {
        self.actor.load(path);
    }
}
