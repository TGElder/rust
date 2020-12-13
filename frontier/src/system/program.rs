use commons::fn_sender::{fn_channel, FnMessageExt, FnReceiver, FnSender};
use commons::futures::future::FutureExt;

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

    pub fn tx(&self) -> &FnSender<Self> {
        &self.tx
    }

    pub async fn run(mut self) -> Self {
        while self.run {
            self.step().await;
        }
        self.run = true;
        self
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

    pub fn shutdown(&mut self) {
        self.run = false;
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
