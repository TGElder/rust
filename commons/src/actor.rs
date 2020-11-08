use crate::Arm;
use async_channel::{unbounded, Receiver, Sender};
use futures::executor::block_on;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub type Action<I, O> = dyn FnOnce(&mut I) -> O + Send + Sync;

pub struct SharedState<I> {
    action: Option<Box<Action<I, ()>>>,
    waker: Option<Waker>,
    sender_handle: &'static str,
}

impl<I> SharedState<I> {
    pub fn sender_handle(&self) -> &'static str {
        &self.sender_handle
    }

    fn try_wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }
}

trait ArmSharedStateExt<I> {
    fn take_action(&self) -> Option<Box<Action<I, ()>>>;
    fn try_wake(&self);
}

impl<I> ArmSharedStateExt<I> for Arm<SharedState<I>> {
    fn take_action(&self) -> Option<Box<Action<I, ()>>> {
        let mut shared_state = self.lock().unwrap();
        shared_state.action.take()
    }

    fn try_wake(&self) {
        let mut shared_state = self.lock().unwrap();
        shared_state.try_wake()
    }
}

pub struct ActorFuture<I, O> {
    shared_state: Arm<SharedState<I>>,
    output: Arm<Option<O>>,
}

impl<I, O> Future for ActorFuture<I, O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        if let Some(output) = self.output.lock().unwrap().take() {
            Poll::Ready(output)
        } else {
            let mut state = self.shared_state.lock().unwrap();
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct Director<I> {
    tx: Sender<Arm<SharedState<I>>>,
    name: &'static str,
}

impl<I> Director<I> {
    pub fn name(&self) -> &'static str {
        &self.name
    }

    pub fn clone_with_name(&self, name: &'static str) -> Director<I> {
        Director {
            tx: self.tx.clone(),
            name,
        }
    }
}

impl<I> Clone for Director<I> {
    fn clone(&self) -> Director<I> {
        Director {
            tx: self.tx.clone(),
            name: self.name,
        }
    }
}

pub trait Direct<I> {
    fn act<O, F>(&self, action: F) -> ActorFuture<I, O>
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

impl<I> Direct<I> for Director<I> {
    fn act<O, F>(&self, action: F) -> ActorFuture<I, O>
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

        let shared_state = SharedState {
            waker: None,
            action: Some(Box::new(action)),
            sender_handle: self.name,
        };
        let shared_state = Arc::new(Mutex::new(shared_state));

        self.tx
            .try_send(shared_state.clone())
            .unwrap_or_else(|err| panic!("Director {} could not send action: {}", self.name, err));

        ActorFuture {
            shared_state,
            output,
        }
    }
}

pub struct Actor<I> {
    rx: Receiver<Arm<SharedState<I>>>,
}

impl<I> Actor<I> {
    pub async fn get_update(&mut self) -> Arm<SharedState<I>> {
        self.rx.recv().await.unwrap()
    }
}

pub trait Act<I> {
    fn act(&mut self, input: &mut I);
}

impl<I> Act<I> for Arm<SharedState<I>> {
    fn act(&mut self, input: &mut I) {
        let mut update = self.lock().unwrap();
        if let Some(function) = update.action.take() {
            function(input);
            if let Some(waker) = update.waker.take() {
                waker.wake()
            }
        }
    }
}

pub fn action_channel<I>() -> (Director<I>, Actor<I>) {
    let (tx, rx) = unbounded();
    (Director { tx, name: "root" }, Actor { rx })
}

#[derive(Clone)]
struct TestDirector<I> {
    state: Arm<Option<I>>,
}

impl<I> TestDirector<I> {
    pub fn new(state: I) -> TestDirector<I> {
        TestDirector {
            state: Arc::new(Mutex::new(Some(state))),
        }
    }

    pub fn take(&self) -> I {
        self.state.lock().unwrap().take().unwrap()
    }
}

impl<I> Direct<I> for TestDirector<I> {
    fn act<O, F>(&self, action: F) -> ActorFuture<I, O>
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        let output = action(self.state.lock().unwrap().as_mut().unwrap());

        ActorFuture {
            shared_state: Arc::new(Mutex::new(SharedState {
                action: None,
                waker: None,
                sender_handle: "test_director",
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
    fn director() {
        struct State {
            value: usize,
            run: bool,
        }

        let (tx, mut rx) = action_channel();

        let mut state = State {
            value: 100,
            run: true,
        };

        let handle = thread::spawn(move || {
            while state.run {
                block_on(rx.get_update()).act(&mut state);
            }
            state.value
        });

        tx.wait(|state| state.value += 1);
        assert_eq!(tx.wait(|state| state.value), 101);

        tx.wait(|state| state.run = false);
        assert_eq!(handle.join().unwrap(), 101);
    }

    #[test]
    fn test_director() {
        let director = TestDirector::new(100usize);

        director.wait(|value| *value += 1);

        assert_eq!(director.wait(|value| *value), 101);
        assert_eq!(director.take(), 101);
    }
}
