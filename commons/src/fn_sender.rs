use crate::Arm;
use async_channel::{unbounded, Receiver, Sender};
use futures::executor::block_on;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub type MessageFn<I, O> = dyn FnOnce(&mut I) -> O + Send + Sync;

pub struct FnMessage<I> {
    function: Option<Box<MessageFn<I, ()>>>,
    waker: Option<Waker>,
    sender_handle: &'static str,
}

impl<I> FnMessage<I> {
    pub fn sender_handle(&self) -> &'static str {
        &self.sender_handle
    }

    fn try_wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }
}

trait PrivateFnMessageExt<I> {
    fn take_function(&self) -> Option<Box<MessageFn<I, ()>>>;
    fn try_wake(&self);
}

impl<I> PrivateFnMessageExt<I> for Arm<FnMessage<I>> {
    fn take_function(&self) -> Option<Box<MessageFn<I, ()>>> {
        let mut message = self.lock().unwrap();
        message.function.take()
    }

    fn try_wake(&self) {
        let mut message = self.lock().unwrap();
        message.try_wake()
    }
}

pub trait FnMessageExt<I> {
    fn apply(&mut self, input: &mut I);
}

impl<I> FnMessageExt<I> for Arm<FnMessage<I>> {
    fn apply(&mut self, input: &mut I) {
        if let Some(function) = self.take_function() {
            function(input);
            self.try_wake();
        }
    }
}

pub struct FnSenderFuture<I, O> {
    message: Arm<FnMessage<I>>,
    output: Arm<Option<O>>,
}

impl<I, O> Future for FnSenderFuture<I, O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        if let Some(output) = self.output.lock().unwrap().take() {
            Poll::Ready(output)
        } else {
            let mut message = self.message.lock().unwrap();
            message.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub trait FnSender<I> {
    fn send<O, F>(&self, function: F) -> FnSenderFuture<I, O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static;

    fn wait<O, F>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        block_on(self.send(function))
    }
}

pub struct FnMessageSender<I> {
    tx: Sender<Arm<FnMessage<I>>>,
    name: &'static str,
}

impl<I> FnMessageSender<I> {
    pub fn name(&self) -> &'static str {
        &self.name
    }

    pub fn clone_with_name(&self, name: &'static str) -> FnMessageSender<I> {
        FnMessageSender {
            tx: self.tx.clone(),
            name,
        }
    }
}

impl<I> Clone for FnMessageSender<I> {
    fn clone(&self) -> FnMessageSender<I> {
        FnMessageSender {
            tx: self.tx.clone(),
            name: self.name,
        }
    }
}

impl<I> FnSender<I> for FnMessageSender<I> {
    fn send<O, F>(&self, function: F) -> FnSenderFuture<I, O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        let output = Arc::new(Mutex::new(None));
        let output_in_fn = output.clone();
        let function = move |input: &mut I| {
            let out = function(input);
            *output_in_fn.lock().unwrap() = Some(out);
        };

        let message = FnMessage {
            waker: None,
            function: Some(Box::new(function)),
            sender_handle: self.name,
        };
        let message = Arc::new(Mutex::new(message));

        self.tx.try_send(message.clone()).unwrap_or_else(|err| {
            panic!(
                "Function sender {} could not send message: {}",
                self.name, err
            )
        });

        FnSenderFuture { message, output }
    }
}

pub struct FnMessageReceiver<I> {
    rx: Receiver<Arm<FnMessage<I>>>,
}

impl<I> FnMessageReceiver<I> {
    pub async fn get_message(&mut self) -> Arm<FnMessage<I>> {
        self.rx
            .recv()
            .await
            .unwrap_or_else(|err| panic!("Function receiver could not receive message: {}", err))
    }
}

pub fn fn_channel<I>() -> (FnMessageSender<I>, FnMessageReceiver<I>) {
    let (tx, rx) = unbounded();
    (
        FnMessageSender { tx, name: "root" },
        FnMessageReceiver { rx },
    )
}

#[derive(Clone)]
struct TestSender<I> {
    state: Arm<Option<I>>,
}

impl<I> TestSender<I> {
    pub fn new(state: I) -> TestSender<I> {
        TestSender {
            state: Arc::new(Mutex::new(Some(state))),
        }
    }

    pub fn take(&self) -> I {
        self.state.lock().unwrap().take().unwrap()
    }
}

impl<I> FnSender<I> for TestSender<I> {
    fn send<O, F>(&self, function: F) -> FnSenderFuture<I, O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        let output = function(self.state.lock().unwrap().as_mut().unwrap());

        FnSenderFuture {
            message: Arc::new(Mutex::new(FnMessage {
                function: None,
                waker: None,
                sender_handle: "test sender",
            })),
            output: Arc::new(Mutex::new(Some(output))),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::thread;

    #[test]
    fn fn_sender() {
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
                block_on(rx.get_message()).apply(&mut state);
            }
            state.value
        });

        tx.wait(|state| state.value += 1);
        assert_eq!(tx.wait(|state| state.value), 101);

        tx.wait(|state| state.run = false);
        assert_eq!(handle.join().unwrap(), 101);
    }

    #[test]
    fn test_sender() {
        let actor = TestSender::new(100usize);

        actor.wait(|value| *value += 1);

        assert_eq!(actor.wait(|value| *value), 101);
        assert_eq!(actor.take(), 101);
    }
}
