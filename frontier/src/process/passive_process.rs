use super::*;

pub struct PassiveProcess<T>
where
    T: Send + 'static,
{
    state: Option<ProcessState<T>>,
}

impl<T> PassiveProcess<T>
where
    T: Send + 'static,
{
    pub fn new(actor: T, actor_rx: FnReceiver<T>) -> PassiveProcess<T> {
        PassiveProcess {
            state: Some(ProcessState::Paused {
                actor,
                rx_state: ReceiverState::accumulating(actor_rx),
            }),
        }
    }
}

#[async_trait]
impl<T> Process for PassiveProcess<T>
where
    T: Send + 'static,
{
    type T = T;

    fn state(&self) -> &Option<ProcessState<Self::T>> {
        &self.state
    }

    fn mut_state(&mut self) -> &mut Option<ProcessState<Self::T>> {
        &mut self.state
    }

    async fn step(t: &mut Program<Self::T>) {
        select! {
            mut message = t.rx.get_message().fuse() => message.apply(t).await,
            mut message = t.actor_rx.get_message().fuse() => message.apply(&mut t.actor).await,
        }
    }
}
