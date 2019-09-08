use std::default::Default;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

pub trait HubEvent: Clone + Send + Sync + 'static {
    fn is_shutdown(&self) -> bool;
}

pub trait Action<E>: Send + 'static
where
    E: HubEvent,
{
    fn act(&mut self, event: &E) -> Vec<E>;
}

pub struct Hub<E>
where
    E: HubEvent,
{
    input_tx: Sender<Arc<E>>,
    input_rx: Receiver<Arc<E>>,
    output_txs: Vec<Sender<Arc<E>>>,
    threads: Vec<JoinHandle<()>>,
}

impl<E> Default for Hub<E>
where
    E: HubEvent,
{
    fn default() -> Hub<E> {
        let (input_tx, input_rx) = mpsc::channel();
        Hub {
            input_tx,
            input_rx,
            output_txs: vec![],
            threads: vec![],
        }
    }
}

impl<E> Hub<E>
where
    E: HubEvent,
{
    pub fn input_tx(&self) -> Sender<Arc<E>> {
        self.input_tx.clone()
    }

    pub fn run(mut self) {
        loop {
            match self.input_rx.recv() {
                Ok(event) => {
                    for output_tx in self.output_txs.iter_mut() {
                        output_tx
                            .send(event.clone())
                            .expect("Hub could not send messages");
                    }
                    if event.is_shutdown() {
                        for thread in self.threads.into_iter() {
                            thread.join().expect("Could not wait for actor to shutdown");
                        }
                        return;
                    }
                }
                Err(err) => panic!("Hub could not receive messages: {:?}", err),
            }
        }
    }

    pub fn add_actor<A>(&mut self, action: A)
    where
        A: Action<E>,
    {
        let (actor_tx, actor_rx) = mpsc::channel();
        let mut actor = Actor {
            action,
            input_rx: actor_rx,
            router_tx: self.input_tx.clone(),
        };
        self.output_txs.push(actor_tx);
        self.threads.push(thread::spawn(move || actor.run()));
    }
}

pub struct Actor<E, A>
where
    E: HubEvent,
    A: Action<E>,
{
    action: A,
    input_rx: Receiver<Arc<E>>,
    router_tx: Sender<Arc<E>>,
}

impl<E, A> Actor<E, A>
where
    E: HubEvent,
    A: Action<E>,
{
    pub fn run(&mut self) {
        loop {
            match self.input_rx.recv() {
                Ok(event) => {
                    for result in self.action.act(&event) {
                        self.router_tx
                            .send(Arc::new(result))
                            .expect("Actor could not send message");
                    }
                    if event.is_shutdown() {
                        return;
                    }
                }
                Err(err) => panic!("Actor could not receive message: {:?}", err),
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Clone)]
    pub enum TestEvent {
        A,
        B,
        Shutdown,
    }

    impl HubEvent for TestEvent {
        fn is_shutdown(&self) -> bool {
            if let TestEvent::Shutdown = self {
                return true;
            }
            false
        }
    }

    struct AtoB {}

    impl Action<TestEvent> for AtoB {
        fn act(&mut self, event: &TestEvent) -> Vec<TestEvent> {
            if let TestEvent::A = event {
                println!("B");
                return vec![TestEvent::B];
            }
            vec![]
        }
    }

    struct B {
        success_tx: Sender<u32>,
    }

    impl Action<TestEvent> for B {
        fn act(&mut self, event: &TestEvent) -> Vec<TestEvent> {
            if let TestEvent::B = event {
                self.success_tx
                    .send(1986)
                    .expect("Could not send success signal");
            }
            vec![]
        }
    }

    #[test]
    fn test_hub() {
        let mut hub = Hub::default();
        let hub_tx = hub.input_tx.clone();

        let (success_tx, success_rx) = mpsc::channel();

        hub.add_actor(AtoB {});
        hub.add_actor(B { success_tx });

        let handle = thread::spawn(move || hub.run());

        hub_tx
            .send(Arc::new(TestEvent::A))
            .expect("Could not initiate test");
        assert_eq!(success_rx.recv().unwrap(), 1986);

        hub_tx
            .send(Arc::new(TestEvent::Shutdown))
            .expect("Could not send shutdown signal");

        handle.join().expect("Could not wait for hub to shutdown");
    }

}
