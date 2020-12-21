use super::*;

#[async_trait]
pub trait Step {
    async fn step(&mut self);
}

pub struct ActiveProcess<T>
where
    T: Step + Send + 'static,
{
    state: Option<ProcessState<T>>,
}

impl<T> ActiveProcess<T>
where
    T: Step + Send + 'static,
{
    pub fn new(actor: T, actor_rx: FnReceiver<T>) -> ActiveProcess<T> {
        ActiveProcess {
            state: Some(ProcessState::Paused {
                actor,
                receiver: ReceiverState::accumulating(actor_rx),
            }),
        }
    }
}

#[async_trait]
impl<T> Process for ActiveProcess<T>
where
    T: Step + Send + 'static,
{
    type T = T;

    fn state(&self) -> &Option<ProcessState<Self::T>> {
        &self.state
    }

    fn mut_state(&mut self) -> &mut Option<ProcessState<Self::T>> {
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
