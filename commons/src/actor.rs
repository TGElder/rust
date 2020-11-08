use crate::Arm;
use async_channel::{unbounded, Receiver, Sender};
use futures::executor::block_on;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub type Action<I, O> = dyn FnOnce(&mut I) -> O + Send + Sync;

pub struct ActionMessage<I> {
    action: Option<Box<Action<I, ()>>>,
    waker: Option<Waker>,
    sender_handle: &'static str,
}

impl<I> ActionMessage<I> {
    pub fn sender_handle(&self) -> &'static str {
        &self.sender_handle
    }

    fn try_wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }
}

pub struct ActionFuture<I, O> {
    message: Arm<ActionMessage<I>>,
    output: Arm<Option<O>>,
}

impl<I, O> Future for ActionFuture<I, O> {
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

pub struct ActionSender<I> {
    tx: Sender<Arm<ActionMessage<I>>>,
    name: &'static str,
}

impl<I> ActionSender<I> {
    pub fn name(&self) -> &'static str {
        &self.name
    }

    pub fn clone_with_name(&self, name: &'static str) -> ActionSender<I> {
        ActionSender {
            tx: self.tx.clone(),
            name,
        }
    }
}

impl<I> Clone for ActionSender<I> {
    fn clone(&self) -> ActionSender<I> {
        ActionSender {
            tx: self.tx.clone(),
            name: self.name,
        }
    }
}

pub trait Actor<I> {
    fn act<O, F>(&self, action: F) -> ActionFuture<I, O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static;

    fn wait<O, F>(&self, action: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        block_on(self.act(action))
    }
}

impl<I> Actor<I> for ActionSender<I> {
    fn act<O, F>(&self, action: F) -> ActionFuture<I, O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        let output = Arc::new(Mutex::new(None));
        let output_in_fn = output.clone();
        let action = move |input: &mut I| {
            let out = action(input);
            *output_in_fn.lock().unwrap() = Some(out);
        };

        let message = ActionMessage {
            waker: None,
            action: Some(Box::new(action)),
            sender_handle: self.name,
        };
        let message = Arc::new(Mutex::new(message));

        self.tx.try_send(message.clone()).unwrap_or_else(|err| {
            panic!(
                "Action sender {} could not send message: {}",
                self.name, err
            )
        });

        ActionFuture { message, output }
    }
}

pub struct ActionReceiver<I> {
    rx: Receiver<Arm<ActionMessage<I>>>,
}

impl<I> ActionReceiver<I> {
    pub async fn get_message(&mut self) -> Arm<ActionMessage<I>> {
        self.rx
            .recv()
            .await
            .unwrap_or_else(|err| panic!("Action receiver could not receive message: {}", err))
    }
}

trait PrivateActionMessageExt<I> {
    fn take_action(&self) -> Option<Box<Action<I, ()>>>;
    fn try_wake(&self);
}

impl<I> PrivateActionMessageExt<I> for Arm<ActionMessage<I>> {
    fn take_action(&self) -> Option<Box<Action<I, ()>>> {
        let mut message = self.lock().unwrap();
        message.action.take()
    }

    fn try_wake(&self) {
        let mut message = self.lock().unwrap();
        message.try_wake()
    }
}

pub trait ActionMessageExt<I> {
    fn act(&mut self, input: &mut I);
}

impl<I> ActionMessageExt<I> for Arm<ActionMessage<I>> {
    fn act(&mut self, input: &mut I) {
        if let Some(function) = self.take_action() {
            function(input);
            self.try_wake();
        }
    }
}

pub fn action_channel<I>() -> (ActionSender<I>, ActionReceiver<I>) {
    let (tx, rx) = unbounded();
    (ActionSender { tx, name: "root" }, ActionReceiver { rx })
}

#[derive(Clone)]
struct TestActor<I> {
    state: Arm<Option<I>>,
}

impl<I> TestActor<I> {
    pub fn new(state: I) -> TestActor<I> {
        TestActor {
            state: Arc::new(Mutex::new(Some(state))),
        }
    }

    pub fn take(&self) -> I {
        self.state.lock().unwrap().take().unwrap()
    }
}

impl<I> Actor<I> for TestActor<I> {
    fn act<O, F>(&self, action: F) -> ActionFuture<I, O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        let output = action(self.state.lock().unwrap().as_mut().unwrap());

        ActionFuture {
            message: Arc::new(Mutex::new(ActionMessage {
                action: None,
                waker: None,
                sender_handle: "test_actor",
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
    fn action_sender() {
        struct State {
            value: usize,
            run: bool,
        }

        let mut state = State {
            value: 100,
            run: true,
        };

        let (tx, mut rx) = action_channel();

        let handle = thread::spawn(move || {
            while state.run {
                block_on(rx.get_message()).act(&mut state);
            }
            state.value
        });

        tx.wait(|state| state.value += 1);
        assert_eq!(tx.wait(|state| state.value), 101);

        tx.wait(|state| state.run = false);
        assert_eq!(handle.join().unwrap(), 101);
    }

    #[test]
    fn test_actor() {
        let actor = TestActor::new(100usize);

        actor.wait(|value| *value += 1);

        assert_eq!(actor.wait(|value| *value), 101);
        assert_eq!(actor.take(), 101);
    }
}
