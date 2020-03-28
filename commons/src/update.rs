use crate::Arm;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

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
    tx: SyncSender<Arm<Update<I>>>,
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

        self.tx.try_send(update.clone()).unwrap();

        UpdateFuture { update, output }
    }
}

pub struct UpdateReceiver<I> {
    rx: Receiver<Arm<Update<I>>>,
}

impl<I> UpdateReceiver<I> {
    pub fn get_update(&mut self) -> Result<Arm<Update<I>>, TryRecvError> {
        self.rx.try_recv()
    }

    pub fn get_updates(&mut self) -> Vec<Arm<Update<I>>> {
        let mut out = vec![];
        while let Ok(update) = self.get_update() {
            out.push(update);
        }
        out
    }
}

pub fn process_updates<I>(mut updates: Vec<Arm<Update<I>>>, input: &mut I) {
    for update in updates.drain(..) {
        process_update(update, input);
    }
}

pub fn process_update<I>(update: Arm<Update<I>>, input: &mut I) {
    let mut update = update.lock().unwrap();
    if let Some(function) = update.function.take() {
        function(input);
        if let Some(waker) = update.waker.take() {
            waker.wake()
        }
    }
}

pub fn update_channel<I>(bound: usize) -> (UpdateSender<I>, UpdateReceiver<I>) {
    let (tx, rx) = mpsc::sync_channel(bound);
    (UpdateSender { tx, handle: "root" }, UpdateReceiver { rx })
}
