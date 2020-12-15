use std::sync::Arc;

use commons::async_channel::{Receiver, RecvError};
use commons::futures::executor::ThreadPool;
use commons::futures::future::FutureExt;
use commons::log::info;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};

use crate::actors::{
    ObjectBuilder, TownHouseArtist, TownLabelArtist, VisibilityActor, Voyager, WorldArtistActor,
};
use crate::polysender::Polysender;
use crate::simulation::Simulation;
use crate::system::{ActiveProcess, PassiveProcess, Persistable, Process};
use crate::traits::{SendGame, SendGameState};

const SAVE_PATH: &str = "save";

pub struct System {
    x: Polysender,
    engine_rx: Receiver<Arc<Event>>,
    pool: ThreadPool,
    processes: Processes,
    bindings: Bindings,
    paused: bool,
    run: bool,
}

pub struct Processes {
    pub object_builder: PassiveProcess<ObjectBuilder<Polysender>>,
    pub simulation: ActiveProcess<Simulation<Polysender>>,
    pub town_house_artist: PassiveProcess<TownHouseArtist<Polysender>>,
    pub town_label_artist: PassiveProcess<TownLabelArtist<Polysender>>,
    pub visibility: PassiveProcess<VisibilityActor<Polysender>>,
    pub voyager: PassiveProcess<Voyager<Polysender>>,
    pub world_artist: PassiveProcess<WorldArtistActor<Polysender>>,
}

struct Bindings {
    pause: Button,
    save: Button,
}

impl System {
    pub fn new(
        x: Polysender,
        engine_rx: Receiver<Arc<Event>>,
        pool: ThreadPool,
        processes: Processes,
    ) -> System {
        System {
            x,
            engine_rx,
            pool,
            processes,
            bindings: Bindings {
                pause: Button::Key(VirtualKeyCode::Space),
                save: Button::Key(VirtualKeyCode::P),
            },
            paused: false,
            run: true,
        }
    }

    pub async fn run(&mut self) {
        self.send_init_messages();
        self.start();
        while self.run {
            select! {
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
        info!("Shut down system");
        info!("Shutting down game");
        self.x.send_game(|game| game.shutdown()).await;
        info!("Shut down game");
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
        self.processes.simulation.load(path);
        self.processes.visibility.load(path);
    }

    async fn save(&mut self, path: &str) {
        info!("Saving");
        let already_paused = self.paused;
        if !already_paused {
            self.pause().await;
        }

        self.processes.simulation.save(path);
        self.processes.visibility.save(path);

        let path = path.to_string();
        self.x.send_game(|game| game.save(path)).await;

        if !already_paused {
            self.start();
        }
        info!("Saved");
    }

    fn send_init_messages(&self) {
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

    fn start(&mut self) {
        info!("Starting system");
        self.processes.world_artist.start(&self.pool);
        self.processes.voyager.start(&self.pool);
        self.processes.visibility.start(&self.pool);
        self.processes.town_house_artist.start(&self.pool);
        self.processes.town_label_artist.start(&self.pool);
        self.processes.simulation.start(&self.pool);
        self.processes.object_builder.start(&self.pool);
        self.paused = false;
        info!("Started system");
    }

    async fn pause(&mut self) {
        info!("Pausing system");
        self.processes.object_builder.pause().await;
        self.processes.simulation.pause().await;
        self.processes.town_label_artist.pause().await;
        self.processes.town_house_artist.pause().await;
        self.processes.visibility.pause().await;
        self.processes.voyager.pause().await;
        self.processes.world_artist.pause().await;
        self.paused = true;
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
            self.x
                .send_game_state(|game_state| game_state.speed = game_state.params.default_speed)
                .await;

            self.start();
        } else {
            self.pause().await;

            self.x
                .send_game_state(|game_state| game_state.speed = 0.0)
                .await;
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
