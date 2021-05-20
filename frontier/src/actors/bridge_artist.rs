use crate::artists::BridgeArtist;
use crate::bridge::Bridge;
use crate::traits::{SendEngineCommands, WithBridges, WithWorld};
use commons::edge::Edge;
use isometric::Command;

pub struct BridgeArtistActor<T> {
    cx: T,
    artist: BridgeArtist,
}

impl<T> BridgeArtistActor<T>
where
    T: SendEngineCommands + WithBridges + WithWorld + Send + Sync,
{
    pub fn new(cx: T, artist: BridgeArtist) -> BridgeArtistActor<T> {
        BridgeArtistActor { cx, artist }
    }

    pub async fn init(&self) {
        self.draw_all().await;
    }

    pub async fn draw_all(&self) {
        for bridge in self.bridges().await {
            self.draw_bridge(bridge).await;
        }
    }

    async fn bridges(&self) -> Vec<Bridge> {
        self.cx
            .with_bridges(|bridges| bridges.values().cloned().collect())
            .await
    }

    pub async fn draw_bridge(&self, bridge: Bridge) {
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
