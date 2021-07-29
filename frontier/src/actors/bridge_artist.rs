use crate::artists::BridgeArtist;
use crate::bridges::Bridge;
use crate::bridges::BridgeType::Built;
use crate::traits::{BuiltBridges, SendEngineCommands};

pub struct BridgeArtistActor<T> {
    cx: T,
    artist: BridgeArtist,
}

impl<T> BridgeArtistActor<T>
where
    T: BuiltBridges + SendEngineCommands + Send + Sync,
{
    pub fn new(cx: T, artist: BridgeArtist) -> BridgeArtistActor<T> {
        BridgeArtistActor { cx, artist }
    }

    pub async fn init(&self) {
        self.draw_all().await;
    }

    pub async fn draw_all(&self) {
        for (_, bridges) in self.cx.built_bridges().await {
            for bridge in bridges {
                self.draw_bridge(bridge).await;
            }
        }
    }

    pub async fn draw_bridge(&self, bridge: Bridge) {
        if bridge.bridge_type != Built {
            return;
        }
        let commands = self.artist.draw_bridge(&bridge);
        self.cx.send_engine_commands(commands).await;
    }

    pub async fn erase_bridge(&self, bridge: Bridge) {
        let commands = self.artist.erase_bridge(&bridge);
        self.cx.send_engine_commands(commands).await;
    }
}
