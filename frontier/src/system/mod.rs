mod controller;
mod event_forwarder;
mod init;
mod polysender;
#[allow(clippy::module_inception)]
mod system;

use commons::async_channel::Receiver;
use commons::fn_sender::{FnMessageExt, FnReceiver};
pub use controller::*;
pub use event_forwarder::*;
use futures::future::RemoteHandle;
use futures::FutureExt;
pub use init::*;
pub use polysender::Polysender;
pub use system::*;

pub fn run_system(
    mut system: System,
    mut system_rx: FnReceiver<System>,
    shutdown_rx: Receiver<()>,
) -> RemoteHandle<()> {
    let pool = system.pool.clone(); // TODO weird
    let (runnable, handle) = async move {
        system.start().await;
        loop {
            select! {
            mut message = shutdown_rx.recv().fuse() => {
                system_rx.get_messages().apply(&mut system).await;
                return;
            },
            mut message = system_rx.get_message().fuse() => message.apply(&mut system).await,}
        }
    }
    .remote_handle();

    pool.spawn_ok(runnable);
    handle
}
