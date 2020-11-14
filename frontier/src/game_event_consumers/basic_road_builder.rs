use super::*;
use crate::actors::UpdateRoads;
use crate::travel_duration::TravelDuration;
use commons::edge::Edge;
use commons::futures::FutureExt;
use isometric::{Button, ElementState, ModifiersState, VirtualKeyCode};

const HANDLE: &str = "basic_road_builder";

pub struct BasicRoadBuilder {
    game_tx: FnSender<Game>,
    update_roads_tx: FnSender<UpdateRoads>,
    thread_pool: ThreadPool,
    travel_duration: Option<AutoRoadTravelDuration>,
    binding: Button,
}

impl BasicRoadBuilder {
    pub fn new(
        game_tx: &FnSender<Game>,
        update_roads_tx: &FnSender<UpdateRoads>,
        thread_pool: ThreadPool,
    ) -> BasicRoadBuilder {
        BasicRoadBuilder {
            game_tx: game_tx.clone_with_name(HANDLE),
            update_roads_tx: update_roads_tx.clone_with_name(HANDLE),
            thread_pool,
            travel_duration: None,
            binding: Button::Key(VirtualKeyCode::R),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.travel_duration = Some(AutoRoadTravelDuration::from_params(
            &game_state.params.auto_road_travel,
        ));
    }

    fn build_road(&mut self, game_state: &GameState) {
        if let Some(travel_duration) = &self.travel_duration {
            if let Some(Avatar { name, state, .. }) = &game_state.selected_avatar() {
                if let Some(path) = state.forward_path() {
                    if travel_duration
                        .get_duration(&game_state.world, &path[0], &path[1])
                        .is_some()
                    {
                        let edge = Edge::new(path[0], path[1]);
                        let mode = if game_state.world.is_road(&edge) {
                            RoadBuildMode::Demolish
                        } else {
                            RoadBuildMode::Build
                        };
                        let result = RoadBuilderResult::new(vec![path[0], path[1]], mode);
                        let start_at = game_state.game_micros;
                        let name = name.clone();

                        let update_roads_tx = self.update_roads_tx.clone();
                        let game_tx = self.game_tx.clone();

                        self.thread_pool.spawn_ok(async move {
                            update_roads_tx
                                .send_future(|update_roads| {
                                    update_roads.update_roads(result).boxed()
                                })
                                .await;
                            game_tx.send(move |game| {
                                game.walk_positions(name, path, start_at, None, None);
                            });
                        });
                    }
                }
            }
        }
    }
}

impl GameEventConsumer for BasicRoadBuilder {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Init = event {
            self.init(game_state);
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers: ModifiersState { alt: false, .. },
            ..
        } = *event
        {
            if button == &self.binding {
                self.build_road(game_state);
            }
        }
        CaptureEvent::No
    }
}
