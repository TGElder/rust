use std::sync::Arc;

use commons::async_channel::{Receiver, RecvError};
use commons::async_trait::async_trait;
use commons::futures::executor::ThreadPool;
use commons::futures::future::FutureExt;
use commons::log::info;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};

use crate::actors::{
    BasicRoadBuilder, ObjectBuilder, TownBuilderActor, TownHouseArtist, TownLabelArtist,
    VisibilityActor, Voyager, WorldArtistActor,
};
use crate::polysender::Polysender;
use crate::simulation::Simulation;
use crate::system::{ActiveProcess, PassiveProcess, Persistable, Process};
use crate::traits::{SendGame, SendGameState};

const SAVE_PATH: &str = "save";

pub struct System<T> {
    engine_rx: Receiver<Arc<Event>>,
    pool: ThreadPool,
    kernel: T,
    bindings: Bindings,
    paused: bool,
    run: bool,
}

pub struct Processes {
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

impl Processes {
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

struct Bindings {
    pause: Button,
    save: Button,
}

#[async_trait]
pub trait Kernel {
    async fn start(&mut self, pool: &ThreadPool);
    async fn pause(&mut self);
    async fn save(&mut self, path: &str);
}

#[async_trait]
impl Kernel for Processes {
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

impl<T> System<T>
where
    T: Kernel,
{
    pub fn new(engine_rx: Receiver<Arc<Event>>, pool: ThreadPool, kernel: T) -> System<T> {
        System {
            engine_rx,
            pool,
            kernel,
            bindings: Bindings {
                pause: Button::Key(VirtualKeyCode::Space),
                save: Button::Key(VirtualKeyCode::P),
            },
            paused: false,
            run: true,
        }
    }

    pub async fn run(&mut self) {
        self.start().await;
        while self.run {
            select! {
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
        info!("Shut down system");
    }

    async fn save(&mut self, path: &str) {
        info!("Saving");
        let already_paused = self.paused;
        if !already_paused {
            self.pause().await;
        }

        self.kernel.save(path).await;

        if !already_paused {
            self.start().await;
        }
        info!("Saved");
    }

    async fn start(&mut self) {
        info!("Starting system");
        self.kernel.start(&self.pool).await;
        self.paused = false;
        info!("Started system");
    }

    async fn pause(&mut self) {
        info!("Pausing system");
        self.kernel.pause().await;
        info!("Paused system");
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        let event: Arc<Event> = event.unwrap();

        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers:
                ModifiersState {
                    alt: false,
                    ctrl: true,
                    ..
                },
            ..
        } = *event
        {
            if button == &self.bindings.pause {
                self.toggle_pause().await;
            } else if button == &self.bindings.save {
                self.save(SAVE_PATH).await;
            }
        }
        if let Event::Shutdown = *event {
            self.shutdown().await;
        }
    }

    async fn toggle_pause(&mut self) {
        if self.paused {
            self.start().await;
        } else {
            self.pause().await;
        }
    }

    async fn shutdown(&mut self) {
        info!("Shutting down system");
        if !self.paused {
            self.pause().await;
        }
        self.run = false;
    }
}
