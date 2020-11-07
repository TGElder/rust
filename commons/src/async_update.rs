use crate::Arm;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use futures::executor::block_on;
use async_channel::{unbounded, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::thread::JoinHandle;

pub type UpdateFn<I, O> = dyn FnOnce(&mut I) -> O + Send + Sync;

pub struct Update<I> {
    waker: Option<Waker>,
    function: Option<Box<UpdateFn<I, ()>>>,
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
            function: Some(Box::new(function)),
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
    rx: Receiver<Arm<Update<I>>>,
}

impl<I> UpdateReceiver<I> {
    pub async fn get_update(&mut self) -> Arm<Update<I>> {
        self.rx.recv().await.unwrap()
    }
}

pub trait Process<I>{
    fn process(&mut self, input: &mut I);
}

impl <I> Process<I> for Arm<Update<I>> {
    fn process(&mut self, input: &mut I) {
        let mut update = self.lock().unwrap();
        if let Some(function) = update.function.take() {
            function(input);
            if let Some(waker) = update.waker.take() {
                waker.wake()
            }
        }
    }
}

pub fn update_channel<I>() -> (UpdateSender<I>, UpdateReceiver<I>) {
    let (tx, rx) = unbounded();
    (UpdateSender { tx, handle: "root" }, UpdateReceiver { rx })
}

pub struct UpdateProcess<I> {
    tx: UpdateSender<I>,
    handle: JoinHandle<I>,
    run: Arc<Mutex<AtomicBool>>,
}

impl<I> UpdateProcess<I>
where
    I: Send + 'static,
{
    pub fn new(mut t: I) -> UpdateProcess<I> {
        let (tx, mut rx) = update_channel();
        let run = Arc::new(Mutex::new(AtomicBool::new(true)));
        let run_2 = run.clone();
        let handle = thread::spawn(move || block_on(async {
            while run_2.lock().unwrap().load(Ordering::Relaxed) {
                rx.get_update().await.process(&mut t);
            }
            t
        }));
        UpdateProcess { tx, handle, run }
    }

    pub fn tx(&self) -> &UpdateSender<I> {
        &self.tx
    }

    pub fn shutdown(self) -> I {
        self.run.lock().unwrap().store(false, Ordering::Relaxed);
        self.handle.join().unwrap()
    }
}
