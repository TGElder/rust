use super::*;

#[async_trait]
pub trait Step {
    async fn step(&mut self);
}

pub struct ActiveProcess<T>
where
    T: Step + Send + 'static,
{
    state: ProcessState<T>,
}

impl<T> ActiveProcess<T>
where
    T: Step + Send + 'static,
{
    pub fn new(actor: T, actor_rx: FnReceiver<T>) -> ActiveProcess<T> {
        let (tx, rx) = fn_channel();
        let program = Program {
            actor,
            actor_rx,
            tx,
            rx,
            run: true,
        };
        ActiveProcess {
            state: ProcessState::Paused(Some(program)),
        }
    }
}

#[async_trait]
impl<T> Process for ActiveProcess<T>
where
    T: Step + Send + 'static,
{
    type T = T;

    fn state(&self) -> &ProcessState<Self::T> {
        &self.state
    }

    fn mut_state(&mut self) -> &mut ProcessState<Self::T> {
        &mut self.state
    }

    async fn step(t: &mut Program<Self::T>) {
        t.rx.get_messages().apply(t).await;
        if !t.run {
            return;
        }
        t.actor_rx.get_messages().apply(&mut t.actor).await;
        t.actor.step().await;
    }
}
