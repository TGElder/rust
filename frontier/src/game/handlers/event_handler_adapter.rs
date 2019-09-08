use super::*;
use isometric::EventHandler;

pub struct EventHandlerAdapter<T>
where
    T: EventHandler,
{
    event_handler: T,
    command_tx: Sender<GameCommand>,
}

impl<T> EventHandlerAdapter<T>
where
    T: EventHandler,
{
    pub fn new(event_handler: T, command_tx: Sender<GameCommand>) -> EventHandlerAdapter<T> {
        EventHandlerAdapter {
            event_handler,
            command_tx,
        }
    }

    fn handle_event(&mut self, event: Arc<Event>) {
        let commands = self.event_handler.handle_event(event);
        self.command_tx
            .send(GameCommand::EngineCommands(commands))
            .unwrap();
    }
}

impl<T> GameEventConsumer for EventHandlerAdapter<T>
where
    T: EventHandler,
{
    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        self.handle_event(event);
        CaptureEvent::No
    }
}
