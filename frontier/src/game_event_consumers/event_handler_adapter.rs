use super::*;
use isometric::EventHandler;

const HANDLE: &str = "event_handler_adapter";

pub struct EventHandlerAdapter<T>
where
    T: EventHandler,
{
    event_handler: T,
    game_tx: FnSender<Game>,
}

impl<T> EventHandlerAdapter<T>
where
    T: EventHandler,
{
    pub fn new(event_handler: T, game_tx: &FnSender<Game>) -> EventHandlerAdapter<T> {
        EventHandlerAdapter {
            event_handler,
            game_tx: game_tx.clone_with_name(HANDLE),
        }
    }

    fn handle_event(&mut self, event: Arc<Event>) {
        let commands = self.event_handler.handle_event(event);
        if !commands.is_empty() {
            self.game_tx
                .send(move |game| game.send_engine_commands(commands));
        }
    }
}

impl<T> GameEventConsumer for EventHandlerAdapter<T>
where
    T: EventHandler,
{
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        self.handle_event(event);
        CaptureEvent::No
    }
}
