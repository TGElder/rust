use crate::Arm;
use async_channel::{unbounded, Receiver, Sender};
use futures::executor::block_on;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub type Fn<I, O> = dyn FnOnce(&mut I) -> O + Send + Sync;

enum Command<I> {
    Act(Box<Fn<I, ()>>),
    Shutdown(Arm<Option<I>>),
}

pub enum Handle {
    Director(&'static str),
    Terminator,
}

pub struct SharedState<I> {
    command: Option<Command<I>>,
    waker: Option<Waker>,
    sender_handle: Handle,
}

impl<I> SharedState<I> {
    pub fn sender_handle(&self) -> &Handle {
        &self.sender_handle
    }

    pub fn try_wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }
}

pub struct ActorFuture<I, O> {
    shared_state: Arm<SharedState<I>>,
    output: Arm<Option<O>>,
}

impl<I, O> Future for ActorFuture<I, O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        let mut state = self.shared_state.lock().unwrap();
        if let Some(output) = self.output.lock().unwrap().take() {
            Poll::Ready(output)
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct Director<I> {
    tx: Sender<Arm<SharedState<I>>>,
    handle: &'static str,
}

impl<T> Director<T> {
    pub fn handle(&self) -> &'static str {
        &self.handle
    }

    pub fn clone_with_handle(&self, handle: &'static str) -> Director<T> {
        Director {
            tx: self.tx.clone(),
            handle,
        }
    }
}

impl<I> Director<I> {
    pub fn act<O, F>(&self, function: F) -> ActorFuture<I, O>
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
        let shared_state = SharedState {
            waker: None,
            command: Some(Command::Act(Box::new(function))),
            sender_handle: Handle::Director(self.handle),
        };
        let shared_state = Arc::new(Mutex::new(shared_state));

        self.tx
            .try_send(shared_state.clone())
            .unwrap_or_else(|err| panic!("{} could not send message: {}", self.handle, err));

        ActorFuture {
            shared_state,
            output,
        }
    }

    pub fn wait<O, F>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        block_on(async { self.act(function).await })
    }
}

pub struct Terminator<I> {
    tx: Sender<Arm<SharedState<I>>>,
}

impl<I> Terminator<I> {
    pub fn terminate(self) -> ActorFuture<I, I> {
        let output = Arc::new(Mutex::new(None));
        let shared_state = SharedState {
            waker: None,
            command: Some(Command::Shutdown(output.clone())),
            sender_handle: Handle::Terminator,
        };
        let shared_state = Arc::new(Mutex::new(shared_state));

        self.tx
            .try_send(shared_state.clone())
            .unwrap_or_else(|err| panic!("Could not terminate: {}", err));

        ActorFuture {
            shared_state,
            output,
        }
    }

    pub fn terminate_and_wait(self) -> I {
        block_on(async { self.terminate().await })
    }
}

pub struct Actor<I> {
    state: I,
    tx: Sender<Arm<SharedState<I>>>,
    rx: Receiver<Arm<SharedState<I>>>,
}

impl<I> Actor<I> {
    pub fn new(state: I) -> (Actor<I>, Terminator<I>) {
        let (tx, rx) = unbounded();
        let terminator = Terminator { tx: tx.clone() };
        (Actor { state, tx, rx }, terminator)
    }

    pub fn director(&self, handle: &'static str) -> Director<I> {
        Director {
            tx: self.tx.clone(),
            handle,
        }
    }

    pub async fn run(mut self) {
        loop {
            let state = self.rx.recv().await.unwrap();
            let mut update = state.lock().unwrap();
            if let Some(function) = update.command.take() {
                match function {
                    Command::Act(function) => {
                        function(&mut self.state);
                        update.try_wake();
                    }
                    Command::Shutdown(output) => {
                        *output.lock().unwrap() = Some(self.state);
                        update.try_wake();
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::ThreadPoolBuilder;
    use std::time::{Duration, Instant};

    fn test(actor_count: usize, pool_size: usize) {
        let max_wait = Duration::from_secs(10);
        let directions = 10;

        let mut actors = vec![];
        let mut terminators = vec![];
        let mut spawning_directors = vec![];
        let mut waiting_directors = vec![];

        for _ in 0..actor_count {
            let (actor, terminator) = Actor::new(0usize);
            spawning_directors.push(actor.director(""));
            waiting_directors.push(actor.director(""));
            actors.push(actor);
            terminators.push(terminator);
        }

        let pool = ThreadPoolBuilder::new()
            .pool_size(pool_size)
            .name_prefix("pool")
            .create()
            .unwrap();

        for actor in actors {
            pool.spawn_ok(actor.run());
        }

        for _ in 0..directions {
            for (_, director) in spawning_directors.iter().enumerate() {
                let director = director.clone_with_handle("test");
                pool.spawn_ok(async move {
                    director.act(move |value| *value += 1);
                });
            }
        }

        let start = Instant::now();
        for director in waiting_directors.iter_mut() {
            while director.wait(move |value| *value < directions) {
                if start.elapsed() > max_wait {
                    panic!("Still waiting after {:?}", max_wait);
                }
            }
        }

        for terminator in terminators {
            assert_eq!(terminator.terminate_and_wait(), directions);
        }
    }

    #[test]
    fn test_single_threaded() {
        test(8, 1);
    }

    #[test]
    fn test_multi_threaded() {
        test(8, 2);
    }
}
