use super::*;
use crate::travel_duration::TravelDuration;
use commons::edge::*;
use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub struct NaturalRoadBuilderParams {
    visitor_count_threshold: usize,
}

impl Default for NaturalRoadBuilderParams {
    fn default() -> NaturalRoadBuilderParams {
        NaturalRoadBuilderParams {
            visitor_count_threshold: 3,
        }
    }
}

pub struct NaturalRoadBuilder {
    command_tx: Sender<GameCommand>,
    visitors: HashMap<Edge, HashSet<String>>,
    state: Option<NaturalRoadBuilderState>,
    params: NaturalRoadBuilderParams,
}

struct NaturalRoadBuilderState {
    travel_mode_fn: TravelModeFn,
    travel_duration: AutoRoadTravelDuration,
}

impl NaturalRoadBuilder {
    pub fn new(command_tx: Sender<GameCommand>) -> NaturalRoadBuilder {
        NaturalRoadBuilder {
            command_tx,
            visitors: HashMap::new(),
            state: None,
            params: NaturalRoadBuilderParams::default(),
        }
    }

    fn init(&mut self, game_state: &GameState) {
        self.state = Some(NaturalRoadBuilderState {
            travel_mode_fn: TravelModeFn::new(
                game_state.params.avatar_travel.min_navigable_river_width,
            ),
            travel_duration: AutoRoadTravelDuration::from_params(
                &game_state.params.auto_road_travel,
            ),
        });
    }

    fn handle_traffic(&mut self, game_state: &GameState, avatar: String, positions: &[V2<usize>]) {
        for i in 0..positions.len() - 1 {
            let edge = Edge::new(positions[i], positions[i + 1]);
            if self.should_record_traffic(game_state, &edge) {
                let visitors = self.visitors.entry(edge).or_insert_with(HashSet::new);
                visitors.insert(avatar.clone());
                let visitor_count = visitors.len();
                if self.should_build_road(game_state, &edge, visitor_count) {
                    self.build_road(&edge);
                    self.visitors.remove(&edge);
                }
            }
        }
    }

    fn build_road(&mut self, edge: &Edge) {
        self.command_tx
            .send(GameCommand::UpdateRoads(RoadBuilderResult::new(
                vec![*edge.from(), *edge.to()],
                true,
            )))
            .unwrap();
    }

    fn should_record_traffic(&self, game_state: &GameState, edge: &Edge) -> bool {
        if game_state.world.is_road(&edge) {
            return false;
        }
        if let Some(NaturalRoadBuilderState {
            travel_duration, ..
        }) = &self.state
        {
            if travel_duration.get_duration(&game_state.world, edge.from(), edge.to()) != None {
                return true;
            }
        }
        false
    }

    fn should_build_road(&self, game_state: &GameState, edge: &Edge, visitor_count: usize) -> bool {
        if visitor_count < self.params.visitor_count_threshold {
            return false;
        }
        self.is_anchor_point(game_state, edge.from()) || self.is_anchor_point(game_state, edge.to())
    }

    fn is_anchor_point(&self, game_state: &GameState, position: &V2<usize>) -> bool {
        fn is_house_on_cell(cell: &WorldCell) -> bool {
            if let WorldObject::House(..) = cell.object {
                true
            } else {
                false
            }
        }

        fn is_house_adjacent_to_position(world: &World, position: &V2<usize>) -> bool {
            world
                .get_corners_behind(position)
                .iter()
                .flat_map(|corner| world.get_cell(corner))
                .any(|cell| is_house_on_cell(cell))
        }

        let world = &game_state.world;
        let cell = world.get_cell_unsafe(position);
        if cell.road.here() {
            return true;
        }
        if is_house_adjacent_to_position(&game_state.world, position) {
            return true;
        }
        if let Some(NaturalRoadBuilderState { travel_mode_fn, .. }) = &self.state {
            if travel_mode_fn.is_navigable_river_here(&game_state.world, position) {
                return true;
            }
        }
        false
    }

    fn get_path(path: &str) -> String {
        format!("{}.visitors", path)
    }

    fn save(&mut self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.visitors).unwrap();
    }

    fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.visitors = bincode::deserialize_from(file).unwrap();
    }
}

impl GameEventConsumer for NaturalRoadBuilder {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::Traffic { name, positions } => {
                self.handle_traffic(&game_state, name.clone(), positions)
            }
            GameEvent::Save(path) => self.save(&path),
            GameEvent::Load(path) => self.load(&path),
            _ => (),
        };
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
