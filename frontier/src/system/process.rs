use commons::fn_sender::FnSender;
use commons::futures::executor::ThreadPool;
use commons::futures::future::{FutureExt, RemoteHandle};

use crate::system::{Persistable, Program};

pub struct Process<T>
where
    T: Send + 'static,
{
    state: ProcessState<T>,
}

enum ProcessState<T>
where
    T: Send,
{
    Running {
        handle: RemoteHandle<Program<T>>,
        tx: FnSender<Program<T>>,
    },
    Paused(Option<Program<T>>),
}

impl<T> Process<T>
where
    T: Send,
{
    pub fn new(program: Program<T>) -> Process<T> {
        Process {
            state: ProcessState::Paused(Some(program)),
        }
    }

    pub fn start(&mut self, pool: &ThreadPool) {
        if let ProcessState::Paused(program) = &mut self.state {
            let program = program.take().unwrap();
            let tx = program.tx().clone();
            let (runnable, handle) = async move { program.run().await }.remote_handle();
            pool.spawn_ok(runnable);
            self.state = ProcessState::Running { handle, tx };
        } else {
            panic!("Cannot run program: program is not paused!");
        }
    }

    pub async fn pause(&mut self) {
        if let ProcessState::Running { handle, tx } = &mut self.state {
            tx.send(|program| program.shutdown()).await;
            self.state = ProcessState::Paused(Some(handle.await));
        } else {
            panic!("Cannot pause program: program is not running!");
        }
    }
}

impl<T> Process<T>
where
    T: Send + Persistable,
{
    pub fn save(&self, path: &str) {
        if let ProcessState::Paused(program) = &self.state {
            program.as_ref().unwrap().save(path);
        } else {
            panic!("Cannot save program state: program is not paused!");
        }
    }

    pub fn load(&mut self, path: &str) {
        if let ProcessState::Paused(program) = &mut self.state {
            program.as_mut().unwrap().load(path);
        } else {
            panic!("Cannot load program state: program is not paused!");
        }
    }
}
