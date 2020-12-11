use commons::fn_sender::FnSender;
use commons::futures::executor::ThreadPool;
use commons::futures::future::{FutureExt, RemoteHandle};

use crate::system::Program;

pub struct Process<T>
where
    T: Send + 'static,
{
    state: ProcessState<Program<T>>,
}

impl<T> Process<T>
where
    T: Send + 'static,
{
    pub fn new(program: Program<T>) -> Process<T> {
        Process {
            state: ProcessState::Paused(Some(program)),
        }
    }
}

enum ProcessState<T>
where
    T: Send,
{
    Running {
        handle: RemoteHandle<T>,
        tx: FnSender<T>,
    },
    Paused(Option<T>),
}

impl<T> Process<T>
where
    T: Send + Sync + 'static,
{
    pub fn start(&mut self, pool: &ThreadPool) {
        if let ProcessState::Paused(program) = &mut self.state {
            let program = program.take().unwrap();
            let tx = program.tx().clone();
            let (runnable, handle) = async move { program.run().await }.remote_handle();
            pool.spawn_ok(runnable);
            self.state = ProcessState::Running { handle, tx };
        } else {
            panic!("Program is already running!");
        }
    }

    pub async fn pause(&mut self) {
        if let ProcessState::Running { handle, tx } = &mut self.state {
            tx.send(|actor| actor.shutdown()).await;
            self.state = ProcessState::Paused(Some(handle.await));
        } else {
            panic!("Program is not running!");
        }
    }
}
