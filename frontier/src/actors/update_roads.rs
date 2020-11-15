use crate::avatar::AvatarTravelDuration;
use crate::game::Game;
use crate::pathfinder::Pathfinder;
use crate::road_builder::{RoadBuildMode, RoadBuilderResult};
use commons::async_channel::{Receiver, RecvError};
use commons::edge::Edge;
use commons::fn_sender::{fn_channel, FnMessageExt, FnReceiver, FnSender};
use commons::futures::future::FutureExt;
use isometric::Event;
use std::sync::{Arc, RwLock};

use crate::actors::{Visibility, WorldArtistActor};

const NAME: &str = "update_roads";

pub struct UpdateRoads {
    tx: FnSender<UpdateRoads>,
    rx: FnReceiver<UpdateRoads>,
    engine_rx: Receiver<Arc<Event>>,
    game_tx: FnSender<Game>,
    artist_tx: FnSender<WorldArtistActor>,
    visibility_tx: FnSender<Visibility>,
    pathfinders: Vec<Arc<RwLock<Pathfinder<AvatarTravelDuration>>>>,
    run: bool,
}

impl UpdateRoads {
    pub fn new(
        engine_rx: Receiver<Arc<Event>>,
        game_tx: &FnSender<Game>,
        redraw_tx: &FnSender<WorldArtistActor>,
        visibility_tx: &FnSender<Visibility>,
        pathfinders: Vec<Arc<RwLock<Pathfinder<AvatarTravelDuration>>>>,
    ) -> UpdateRoads {
        let (tx, rx) = fn_channel();
        UpdateRoads {
            tx,
            rx,
            engine_rx,
            game_tx: game_tx.clone_with_name(NAME),
            artist_tx: redraw_tx.clone_with_name(NAME),
            visibility_tx: visibility_tx.clone_with_name(NAME),
            pathfinders,
            run: true,
        }
    }

    pub fn tx(&self) -> &FnSender<UpdateRoads> {
        &self.tx
    }

    pub async fn run(&mut self) {
        while self.run {
            select! {
                mut message = self.rx.get_message().fuse() => message.apply(self).await,
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event)
            }
        }
    }

    pub async fn update_roads(&mut self, result: RoadBuilderResult) {
        let result = Arc::new(result);
        let micros = self.update_world_get_micros(result.clone()).await;
        self.redraw(&result, micros);
        self.visit(&result);
        self.update_pathfinder_with_roads(&result);
    }

    async fn update_world_get_micros(&mut self, result: Arc<RoadBuilderResult>) -> u128 {
        self.game_tx
            .send(move |game| update_world_get_micros(game, result))
            .await
    }

    fn redraw(&mut self, result: &Arc<RoadBuilderResult>, micros: u128) {
        for position in result.path().iter().cloned() {
            self.artist_tx
                .send_future(move |artist| artist.redraw_tile_at(position, micros).boxed());
        }
    }

    fn visit(&mut self, result: &Arc<RoadBuilderResult>) {
        let visited = result.path().iter().cloned().collect();
        self.visibility_tx
            .send(|visibility| visibility.check_visibility_and_reveal(visited));
    }

    fn update_pathfinder_with_roads(&mut self, result: &Arc<RoadBuilderResult>) {
        for pathfinder in self.pathfinders.iter().cloned() {
            let result = result.clone();
            self.game_tx.send(move |game| {
                result.update_pathfinder(&game.game_state().world, &mut pathfinder.write().unwrap())
            });
        }
    }

    pub async fn add_road(&mut self, edge: &Edge) {
        if self.is_road(*edge).await {
            return;
        }
        let result = RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Build);
        self.update_roads(result).await;
    }

    pub async fn remove_road(&mut self, edge: &Edge) {
        if !self.is_road(*edge).await {
            return;
        }
        let result =
            RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Demolish);
        self.update_roads(result).await;
    }

    pub async fn is_road(&mut self, edge: Edge) -> bool {
        self.game_tx.send(move |game| is_road(game, edge)).await
    }

    fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        if let Event::Shutdown = *event.unwrap() {
            self.shutdown();
        }
    }

    fn shutdown(&mut self) {
        self.run = false;
    }
}

fn update_world_get_micros(game: &mut Game, result: Arc<RoadBuilderResult>) -> u128 {
    result.update_roads(&mut game.mut_state().world);
    game.game_state().game_micros
}

fn is_road(game: &mut Game, edge: Edge) -> bool {
    game.game_state().world.is_road(&edge)
}
