use crate::Arm;
use async_channel::{unbounded, Receiver, Sender};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub type Action<I, O> = dyn FnOnce(&mut I) -> O + Send + Sync;

enum Command<I> {
    Act(Box<Action<I, ()>>),
    Terminate(Arm<Option<I>>),
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

    fn try_wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }
}

trait ArmSharedStateExt<I> {
    fn take_command(&self) -> Option<Command<I>>;
    fn try_wake(&self);
}

impl<I> ArmSharedStateExt<I> for Arm<SharedState<I>> {
    fn take_command(&self) -> Option<Command<I>> {
        let mut shared_state = self.lock().unwrap();
        shared_state.command.take()
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

impl<I> Director<I> {
    pub fn act<O, F>(&self, action: F) -> ActorFuture<I, O>
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
            command: Some(Command::Act(Box::new(action))),
            sender_handle: Handle::Director(self.name),
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

    pub fn wait<O, F>(&self, action: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut I) -> O + Send + Sync + 'static,
    {
        sync!(self.act(action))
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
            command: Some(Command::Terminate(output.clone())),
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
        sync!(self.terminate())
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
            name: handle,
        }
    }

    pub async fn run(mut self) {
        loop {
            let state = self.rx.recv().await.unwrap();
            if let Some(command) = state.take_command() {
                match command {
                    Command::Act(action) => {
                        action(&mut self.state);
                        state.try_wake();
                    }
                    Command::Terminate(output) => {
                        *output.lock().unwrap() = Some(self.state);
                        state.try_wake();
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
                let director = director.clone();
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
