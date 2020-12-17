use commons::async_trait::async_trait;
use commons::futures::executor::ThreadPool;
use commons::futures::future::FutureExt;

use crate::actors::{
    BasicRoadBuilder, ObjectBuilder, TownBuilderActor, TownHouseArtist, TownLabelArtist,
    VisibilityActor, Voyager, WorldArtistActor,
};
use crate::polysender::Polysender;
use crate::process::{ActiveProcess, PassiveProcess, Persistable, Process};
use crate::simulation::Simulation;
use crate::system::Kernel;
use crate::traits::{SendGame, SendGameState};

pub struct Frontier {
    pub x: Polysender,
    pub basic_road_builder: PassiveProcess<BasicRoadBuilder<Polysender>>,
    pub object_builder: PassiveProcess<ObjectBuilder<Polysender>>,
    pub simulation: ActiveProcess<Simulation<Polysender>>,
    pub town_builder: PassiveProcess<TownBuilderActor<Polysender>>,
    pub town_house_artist: PassiveProcess<TownHouseArtist<Polysender>>,
    pub town_label_artist: PassiveProcess<TownLabelArtist<Polysender>>,
    pub visibility: PassiveProcess<VisibilityActor<Polysender>>,
    pub voyager: PassiveProcess<Voyager<Polysender>>,
    pub world_artist: PassiveProcess<WorldArtistActor<Polysender>>,
}

impl Frontier {
    pub fn send_init_messages(&self) {
        self.x
            .town_house_artist_tx
            .send_future(|town_house_artist| town_house_artist.init().boxed());
        self.x
            .town_label_artist_tx
            .send_future(|town_label_artist| town_label_artist.init().boxed());
        self.x
            .visibility_tx
            .send_future(|visibility| visibility.init().boxed());
        self.x
            .world_artist_tx
            .send_future(|world_artist| world_artist.init().boxed());
    }

    pub fn new_game(&self) {
        self.x
            .simulation_tx
            .send_future(|simulation| simulation.new_game().boxed());
        self.x
            .visibility_tx
            .send_future(|visibility| visibility.new_game().boxed());
    }

    pub fn load(&mut self, path: &str) {
        self.simulation.load(path);
        self.visibility.load(path);
    }
}

#[async_trait]
impl Kernel for Frontier {
    async fn start(&mut self, pool: &ThreadPool) {
        self.x
            .send_game_state(|game_state| game_state.speed = game_state.params.default_speed)
            .await;

        self.world_artist.start(pool);
        self.voyager.start(pool);
        self.visibility.start(pool);
        self.town_house_artist.start(pool);
        self.town_label_artist.start(pool);
        self.town_builder.start(pool);
        self.simulation.start(pool);
        self.object_builder.start(pool);
        self.basic_road_builder.start(pool);
    }

    async fn pause(&mut self) {
        self.basic_road_builder.pause().await;
        self.object_builder.pause().await;
        self.simulation.pause().await;
        self.town_builder.pause().await;
        self.town_label_artist.pause().await;
        self.town_house_artist.pause().await;
        self.visibility.pause().await;
        self.voyager.pause().await;
        self.world_artist.pause().await;

        self.x
            .send_game_state(|game_state| game_state.speed = 0.0)
            .await;
    }

    async fn save(&mut self, path: &str) {
        self.simulation.save(path);
        self.visibility.save(path);

        let path = path.to_string();
        self.x.send_game(|game| game.save(path)).await;
    }
}
