use crate::artists::BridgeArtist;
use crate::bridge::Bridge;
use crate::bridge::BridgeType::Built;
use crate::traits::{BuiltBridges, SendEngineCommands, WithWorld};
use commons::edge::Edge;
use isometric::Command;

pub struct BridgeArtistActor<T> {
    cx: T,
    artist: BridgeArtist,
}

impl<T> BridgeArtistActor<T>
where
    T: BuiltBridges + SendEngineCommands + WithWorld + Send + Sync,
{
    pub fn new(cx: T, artist: BridgeArtist) -> BridgeArtistActor<T> {
        BridgeArtistActor { cx, artist }
    }

    pub async fn init(&self) {
        self.draw_all().await;
    }

    pub async fn draw_all(&self) {
        for (_, bridge) in self.cx.built_bridges().await {
            self.draw_bridge(bridge).await;
        }
    }

    pub async fn draw_bridge(&self, bridge: Bridge) {
        if *bridge.bridge_type() != Built {
            return;
        }
        let commands = self.get_draw_commands(&bridge).await;
        self.cx.send_engine_commands(commands).await;
    }

    async fn get_draw_commands(&self, bridge: &Bridge) -> Vec<Command> {
        self.cx
            .with_world(|world| self.artist.draw_bridge(world, bridge))
            .await
    }

    pub async fn erase_bridge(&self, edge: Edge) {
        let command = self.artist.erase_bridge(&edge);
        self.cx.send_engine_commands(vec![command]).await;
    }
}
