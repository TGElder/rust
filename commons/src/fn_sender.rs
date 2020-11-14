use crate::Arm;
use async_channel::{unbounded, Receiver, Sender};
use async_trait::async_trait;
use futures::executor::block_on;
use futures::future::{BoxFuture, FutureExt};
use futures::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::{self, JoinHandle};

pub type MessageFn<I, O> = dyn FnOnce(&mut I) -> BoxFuture<O> + Send;

pub struct FnMessage<I> {
    function: Option<Box<MessageFn<I, ()>>>,
    waker: Arm<Option<Waker>>,
    sender_name: &'static str,
}

impl<I> FnMessage<I> {
    pub fn sender_name(&self) -> &'static str {
        &self.sender_name
    }

    fn try_wake(&mut self) {
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake()
        }
    }
}

#[async_trait]
pub trait FnMessageExt<I>
where
    I: Send,
{
    async fn apply(&mut self, input: &mut I);
}

#[async_trait]
impl<I> FnMessageExt<I> for FnMessage<I>
where
    I: Send,
{
    async fn apply(&mut self, input: &mut I) {
        if let Some(function) = self.function.take() {
            function(input).await;
            self.try_wake();
        }
    }
}

#[async_trait]
impl<I> FnMessageExt<I> for Vec<FnMessage<I>>
where
    I: Send,
{
    async fn apply(&mut self, input: &mut I) {
        for message in self {
            message.apply(input).await;
        }
    }
}

pub struct FnSenderFuture<O> {
    waker: Arm<Option<Waker>>,
    output: Arm<Option<O>>,
}

impl<O> Future for FnSenderFuture<O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        if let Some(output) = self.output.lock().unwrap().take() {
            Poll::Ready(output)
        } else {
            let mut waker = self.waker.lock().unwrap();
            *waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct FnSender<I>
where
    I: Send,
{
    tx: Sender<FnMessage<I>>,
    name: &'static str,
}

impl<I> FnSender<I>
where
    I: Send,
{
    pub fn name(&self) -> &'static str {
        &self.name
    }

    pub fn clone_with_name(&self, name: &'static str) -> FnSender<I> {
        FnSender {
            tx: self.tx.clone(),
            name,
        }
    }

    pub fn send_future<O, F>(&self, function: F) -> FnSenderFuture<O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> BoxFuture<O> + Send + 'static,
    {
        let output = Arc::new(Mutex::new(None));
        let output_in_fn = output.clone();
        let function: Box<MessageFn<I, ()>> = Box::new(move |input: &mut I| {
            async move {
                let out = function(input).await;
                *output_in_fn.lock().unwrap() = Some(out);
            }
            .boxed()
        });

        let waker = Arc::new(Mutex::new(None));

        let message = FnMessage {
            function: Some(function),
            waker: waker.clone(),
            sender_name: self.name,
        };

        self.tx.try_send(message).unwrap_or_else(|err| {
            panic!(
                "Function sender {} could not send message: {}",
                self.name, err
            )
        });

        FnSenderFuture { waker, output }
    }

    pub fn wait_future<O, F>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> BoxFuture<O> + Send + 'static,
    {
        block_on(self.send_future(function))
    }

    pub fn send<O, F>(&self, function: F) -> FnSenderFuture<O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + 'static,
    {
        self.send_future(|input| async move { function(input) }.boxed())
    }

    pub fn wait<O, F>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        block_on(self.send(function))
    }
}

impl<I> Clone for FnSender<I>
where
    I: Send,
{
    fn clone(&self) -> FnSender<I> {
        FnSender {
            tx: self.tx.clone(),
            name: self.name,
        }
    }
}

pub struct FnReceiver<I> {
    rx: Receiver<FnMessage<I>>,
}

impl<I> FnReceiver<I> {
    pub async fn get_message(&mut self) -> FnMessage<I> {
        self.rx
            .recv()
            .await
            .unwrap_or_else(|err| panic!("Function receiver could not receive message: {}", err))
    }

    pub fn get_messages(&mut self) -> Vec<FnMessage<I>> {
        let mut out = vec![];
        while let Ok(update) = self.rx.try_recv() {
            out.push(update);
        }
        out
    }
}

pub fn fn_channel<I>() -> (FnSender<I>, FnReceiver<I>)
where
    I: Send,
{
    let (tx, rx) = unbounded();
    (FnSender { tx, name: "root" }, FnReceiver { rx })
}

pub struct FnThread<I>
where
    I: Send,
{
    tx: FnSender<I>,
    handle: JoinHandle<I>,
    run: Arc<Mutex<AtomicBool>>,
}

impl<I> FnThread<I>
where
    I: Send + 'static,
{
    pub fn new(mut t: I) -> FnThread<I> {
        let (tx, mut rx) = fn_channel();
        let run = Arc::new(Mutex::new(AtomicBool::new(true)));
        let run_in_thread = run.clone();
        let handle = thread::spawn(move || {
            while run_in_thread.lock().unwrap().load(Ordering::Relaxed) {
                block_on(rx.get_messages().apply(&mut t));
            }
            t
        });
        FnThread { tx, handle, run }
    }

    pub fn tx(&self) -> &FnSender<I> {
        &self.tx
    }

    pub fn join(self) -> I {
        self.run.lock().unwrap().store(false, Ordering::Relaxed);
        self.handle.join().unwrap()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::thread;

    #[test]
    fn wait_future() {
        struct State {
            value: usize,
            run: bool,
        }

        impl State {
            async fn increment_value(&mut self, increment: usize) {
                self.value += increment;
            }

            async fn value(&self) -> usize {
                self.value
            }

            async fn shutdown(&mut self) {
                self.run = false;
            }
        }

        let mut state = State {
            value: 100,
            run: true,
        };

        let (tx, mut rx) = fn_channel();

        let handle = thread::spawn(move || {
            while state.run {
                block_on(async { rx.get_message().await.apply(&mut state).await });
            }
            state.value
        });

        let increment = 1;
        tx.wait_future(move |state| state.increment_value(increment).boxed());
        assert_eq!(tx.wait_future(|state| state.value().boxed()), 101);

        tx.wait_future(|state| state.shutdown().boxed());
        assert_eq!(handle.join().unwrap(), 101);
    }

    #[test]
    fn wait() {
        struct State {
            value: usize,
            run: bool,
        }

        let mut state = State {
            value: 100,
            run: true,
        };

        let (tx, mut rx) = fn_channel();

        let handle = thread::spawn(move || {
            while state.run {
                block_on(async { rx.get_message().await.apply(&mut state).await });
            }
            state.value
        });

        let increment = 1;
        tx.wait(move |state| state.value += increment);
        assert_eq!(tx.wait(|state| state.value), 101);

        tx.wait(|state| state.run = false);
        assert_eq!(handle.join().unwrap(), 101);
    }

    #[test]
    fn fn_thread() {
        let actor = FnThread::new(100usize);
        let tx = actor.tx().clone();

        tx.wait(|value| *value += 1);

        assert_eq!(tx.wait(|value| *value), 101);
        assert_eq!(actor.join(), 101);
    }
}
