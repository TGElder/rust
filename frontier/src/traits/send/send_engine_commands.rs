use commons::async_trait::async_trait;
use isometric::Command;

#[async_trait]
pub trait SendEngineCommands {
    async fn send_engine_commands(&self, commands: Vec<Command>);
}
