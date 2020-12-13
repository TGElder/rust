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
use crate::system::{Process, Program};

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

struct Processes {
    object_builder: Process<ObjectBuilder<Polysender>>,
    town_house_artist: Process<TownHouseArtist<Polysender>>,
    town_label_artist: Process<TownLabelArtist<Polysender>>,
    visibility: Process<VisibilityActor<Polysender>>,
    voyager: Process<Voyager<Polysender>>,
    world_artist: Process<WorldArtistActor<Polysender>>,
}

pub struct Programs {
    pub object_builder: Program<ObjectBuilder<Polysender>>,
    pub town_house_artist: Program<TownHouseArtist<Polysender>>,
    pub town_label_artist: Program<TownLabelArtist<Polysender>>,
    pub visibility: Program<VisibilityActor<Polysender>>,
    pub voyager: Program<Voyager<Polysender>>,
    pub world_artist: Program<WorldArtistActor<Polysender>>,
}

impl Into<Processes> for Programs {
    fn into(self) -> Processes {
        Processes {
            object_builder: Process::new(self.object_builder),
            town_house_artist: Process::new(self.town_house_artist),
            town_label_artist: Process::new(self.town_label_artist),
            visibility: Process::new(self.visibility),
            voyager: Process::new(self.voyager),
            world_artist: Process::new(self.world_artist),
        }
    }
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
        programs: Programs,
    ) -> System {
        System {
            x,
            engine_rx,
            pool,
            processes: programs.into(),
            bindings: Bindings {
                pause: Button::Key(VirtualKeyCode::Space),
                save: Button::Key(VirtualKeyCode::P),
            },
            paused: false,
            run: true,
        }
    }

    pub fn new_game(&self) {
        self.x
            .visibility_tx
            .send_future(|visibility| visibility.new_game().boxed());
    }

    pub fn load(&mut self, path: &str) {
        self.processes.visibility.load(path);
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
        self.processes.object_builder.start(&self.pool);
        self.paused = false;
        info!("Started system");
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
            self.start();
        } else {
            self.pause().await;
        }
    }

    async fn save(&mut self, path: &str) {
        let already_paused = self.paused;
        if !already_paused {
            self.pause().await;
        }

        self.processes.visibility.save(path);

        let path = path.to_string();
        self.x.game_tx.send(|game| game.save(path));

        if !already_paused {
            self.start();
        }
    }

    async fn pause(&mut self) {
        info!("Pausing system");
        self.processes.object_builder.pause().await;
        self.processes.town_label_artist.pause().await;
        self.processes.town_house_artist.pause().await;
        self.processes.visibility.pause().await;
        self.processes.voyager.pause().await;
        self.processes.world_artist.pause().await;
        self.paused = true;
        info!("Paused system");
    }

    async fn shutdown(&mut self) {
        info!("Shutting down system");
        if !self.paused {
            self.pause().await;
        }
        self.run = false;
    }
}
