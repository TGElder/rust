use crate::Arm;
use async_channel::{unbounded, Receiver, Sender};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub type UpdateFn<I, O> = dyn FnOnce(&mut I) -> O + Send + Sync;

enum UpdateCommand<I> {
    Fn(Box<UpdateFn<I, ()>>),
    Shutdown(Arm<Option<I>>),
}

pub struct Update<I> {
    waker: Option<Waker>,
    function: Option<UpdateCommand<I>>,
    sender_handle: &'static str,
}

impl<I> Update<I> {
    pub fn sender_handle(&self) -> &'static str {
        self.sender_handle
    }
}

pub struct UpdateFuture<I, O> {
    update: Arm<Update<I>>,
    output: Arm<Option<O>>,
}

impl<I, O> Future for UpdateFuture<I, O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        let mut update = self.update.lock().unwrap();
        if let Some(output) = self.output.lock().unwrap().take() {
            Poll::Ready(output)
        } else {
            update.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct UpdateSender<I> {
    tx: Sender<Arm<Update<I>>>,
    handle: &'static str,
}

impl<T> Clone for UpdateSender<T> {
    fn clone(&self) -> UpdateSender<T> {
        UpdateSender {
            tx: self.tx.clone(),
            handle: self.handle,
        }
    }
}

impl<T> UpdateSender<T> {
    pub fn handle(&self) -> &'static str {
        &self.handle
    }

    pub fn clone_with_handle(&self, handle: &'static str) -> UpdateSender<T> {
        UpdateSender {
            tx: self.tx.clone(),
            handle,
        }
    }
}

impl<I> UpdateSender<I> {
    pub fn update<O, F>(&self, function: F) -> UpdateFuture<I, O>
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
        let update = Update {
            waker: None,
            function: Some(UpdateCommand::Fn(Box::new(function))),
            sender_handle: self.handle,
        };
        let update = Arc::new(Mutex::new(update));

        self.tx
            .try_send(update.clone())
            .unwrap_or_else(|err| panic!("{} could not send message: {}", self.handle, err));

        UpdateFuture { update, output }
    }

    pub fn shutdown(&self) -> UpdateFuture<I, I> {
        let output = Arc::new(Mutex::new(None));
        let update = Update {
            waker: None,
            function: Some(UpdateCommand::Shutdown(output.clone())),
            sender_handle: self.handle,
        };
        let update = Arc::new(Mutex::new(update));

        self.tx
            .try_send(update.clone())
            .unwrap_or_else(|err| panic!("{} could not send message: {}", self.handle, err));

        UpdateFuture { update, output }
    }
}

pub struct UpdateReceiver<I> {
    t: I,
    rx: Receiver<Arm<Update<I>>>,
}

impl<I> UpdateReceiver<I> {
    pub async fn process_update(mut self) {
        loop {
            let update = self.rx.recv().await.unwrap();
            let mut update = update.lock().unwrap();
            if let Some(function) = update.function.take() {
                match function {
                    UpdateCommand::Fn(function) => {
                        function(&mut self.t);
                        if let Some(waker) = update.waker.take() {
                            waker.wake()
                        }
                    }
                    UpdateCommand::Shutdown(output) => {
                        *output.lock().unwrap() = Some(self.t);
                        if let Some(waker) = update.waker.take() {
                            waker.wake()
                        }
                        return;
                    }
                }
            }
        }
    }
}

pub fn update_channel<I>(t: I) -> (UpdateSender<I>, UpdateReceiver<I>) {
    let (tx, rx) = unbounded();
    (
        UpdateSender { tx, handle: "root" },
        UpdateReceiver { t, rx },
    )
}

pub struct UpdateProcess<I> {
    tx: UpdateSender<I>,
    rx: UpdateReceiver<I>,
}

impl<I> UpdateProcess<I>
where
    I: Send + 'static,
{
    pub fn new(t: I) -> UpdateProcess<I> {
        let (tx, rx) = update_channel(t);
        UpdateProcess { tx, rx }
    }

    pub async fn run(self) {
        self.rx.process_update().await;
    }

    pub fn tx(&self) -> &UpdateSender<I> {
        &self.tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::executor::ThreadPoolBuilder;
    use std::thread;
    use std::thread::current;

    #[test]
    fn test_name() {
        let mut a = UpdateProcess::new(0usize);
        let mut b = UpdateProcess::new(0usize);
        let mut c = UpdateProcess::new(0usize);
        let mut d = UpdateProcess::new(0usize);

        let mut pool = ThreadPoolBuilder::new()
            .pool_size(3)
            .name_prefix("pool")
            .create()
            .unwrap();

        let a_tx = a.tx().clone();
        let b_tx = b.tx().clone();
        let c_tx = c.tx().clone();
        let d_tx = d.tx().clone();
        let a_tx_2 = a.tx().clone();
        let b_tx_2 = b.tx().clone();
        let c_tx_2 = c.tx().clone();
        let d_tx_2 = d.tx().clone();

        pool.spawn_ok(async move { a.run().await });
        pool.spawn_ok(async move { b.run().await });
        pool.spawn_ok(async move { c.run().await });
        pool.spawn_ok(async move { d.run().await });

        let count = 10;

        thread::spawn(move || {
            for i in 0..count {
                thread::sleep_ms(30);
                a_tx_2.update(|a| {
                    *a += 1;
                    println!("a = {} on {}", a, current().name().unwrap());
                });
            }
        });
        thread::spawn(move || {
            for i in 0..count {
                thread::sleep_ms(70);
                b_tx_2.update(|a| {
                    *a += 1;
                    println!("b = {} on {}", a, current().name().unwrap());
                });
            }
        });
        thread::spawn(move || {
            for i in 0..count {
                thread::sleep_ms(90);
                c_tx_2.update(|a| {
                    *a += 1;
                    println!("c = {} on {}", a, current().name().unwrap());
                });
            }
        });
        thread::spawn(move || {
            for i in 0..count {
                thread::sleep_ms(110);
                d_tx_2.update(|a| {
                    *a += 1;
                    println!("d = {} on {}", a, current().name().unwrap());
                });
            }
        });

        while block_on(async { a_tx.update(|a| *a < 10).await }) {}
        println!("Shutting down a");
        println!("Shut down a {}", block_on(async { a_tx.shutdown().await }));
        while block_on(async { b_tx.update(|a| *a < 10).await }) {}
        println!("Shut down b {}", block_on(async { b_tx.shutdown().await }));
        while block_on(async { c_tx.update(|a| *a < 10).await }) {}
        println!("Shut down c {}", block_on(async { c_tx.shutdown().await }));
        while block_on(async { d_tx.update(|a| *a < 10).await }) {}
        println!("Shut down c {}", block_on(async { d_tx.shutdown().await }));
    }
}
