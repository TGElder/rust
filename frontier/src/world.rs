use crate::roadset::*;
use crate::utils::float_ordering;
use isometric::coords::WorldCoord;
use isometric::terrain::*;
use isometric::*;

pub struct World {
    width: usize,
    height: usize,
    terrain: Terrain,
    rivers: RoadSet,
    roads: RoadSet,
    sea_level: f32,
    max_height: f32,
}

impl World {
    const ROAD_WIDTH: f32 = 0.05;

    pub fn new(
        elevations: M<f32>,
        river_nodes: Vec<Node>,
        rivers: Vec<Edge>,
        sea_level: f32,
    ) -> World {
        let (width, height) = elevations.shape();
        let max_height = elevations.max();
        let rivers = World::setup_rivers(width, height, river_nodes, rivers);
        let from = &v2(0, 0);
        let to = &v2(width, height);
        World {
            width,
            height,
            terrain: Terrain::new(
                elevations,
                &rivers.get_nodes(from, to),
                &rivers.get_edges(from, to),
            ),
            rivers,
            roads: RoadSet::new(width, height, World::ROAD_WIDTH),
            sea_level,
            max_height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn terrain(&self) -> &Terrain {
        &self.terrain
    }

    pub fn rivers(&self) -> &RoadSet {
        &self.rivers
    }

    pub fn roads(&self) -> &RoadSet {
        &self.roads
    }

    pub fn sea_level(&self) -> f32 {
        self.sea_level
    }

    pub fn max_height(&self) -> f32 {
        self.max_height
    }

    fn setup_rivers(
        width: usize,
        height: usize,
        river_nodes: Vec<Node>,
        rivers: Vec<Edge>,
    ) -> RoadSet {
        let mut out = RoadSet::new(width, height, 0.0);
        out.set_widths_from_nodes(&river_nodes);
        out.add_roads(&rivers);
        out
    }

    fn get_horizontal_width(&self, position: &V2<usize>) -> f32 {
        self.rivers
            .get_horizontal_width(position)
            .max(self.roads.get_horizontal_width(position))
    }

    fn get_vertical_width(&self, position: &V2<usize>) -> f32 {
        self.rivers
            .get_vertical_width(position)
            .max(self.roads.get_vertical_width(position))
    }

    fn is_river_or_road(&self, edge: &Edge) -> bool {
        self.rivers.is_road(edge) || self.roads.is_road(edge)
    }

    fn get_node(&self, position: &V2<usize>) -> Node {
        let width = self.get_vertical_width(position);
        let height = self.get_horizontal_width(position);
        Node::new(*position, width, height)
    }

    pub fn add_road(&mut self, edge: &Edge) {
        self.roads.add_road(edge);
        self.update_terrain(edge);
    }

    pub fn clear_road(&mut self, edge: &Edge) {
        self.roads.clear_road(edge);
        self.update_terrain(edge);
    }

    pub fn toggle_road(&mut self, edge: &Edge) {
        if self.roads.is_road(edge) {
            self.clear_road(edge);
        } else {
            self.add_road(edge);
        }
        self.update_terrain(edge);
    }

    fn update_terrain(&mut self, edge: &Edge) {
        if self.is_river_or_road(edge) {
            self.terrain.set_edge(edge);
        } else {
            self.terrain.clear_edge(edge);
        }
        self.terrain.set_node(self.get_node(edge.from()));
        self.terrain.set_node(self.get_node(edge.to()));
    }

    pub fn snap(&self, world_coord: WorldCoord) -> WorldCoord {
        let x = world_coord.x.round();
        let y = world_coord.y.round();
        let z = self.terrain.elevations()[(x as usize, y as usize)];
        WorldCoord::new(x, y, z)
    }

    pub fn snap_middle(&self, world_coord: WorldCoord) -> WorldCoord {
        let x = world_coord.x.floor();
        let y = world_coord.y.floor();
        let mut z = 0.0 as f32;
        for dx in 0..2 {
            for dy in 0..2 {
                z = z.max(self.terrain.elevations()[(x as usize + dx, y as usize + dy)])
            }
        }
        WorldCoord::new(x + 0.5, y + 0.5, z)
    }

    pub fn get_corners(&self, position: &V2<usize>) -> [V2<usize>; 4] {
        [
            *position,
            v2(position.x + 1, position.y),
            v2(position.x + 1, position.y + 1),
            v2(position.x, position.y + 1),
        ]
    }

    pub fn get_border(&self, position: &V2<usize>) -> Vec<Edge> {
        let corners = self.get_corners(position);
        (0..4)
            .map(|i| Edge::new(corners[i], corners[(i + 1) % 4]))
            .collect()
    }

    pub fn in_bounds(&self, position: &V2<usize>) -> bool {
        position.x < self.width && position.y < self.height
    }

    pub fn get_elevation(&self, position: &V2<usize>) -> Option<f32> {
        if self.in_bounds(&position) {
            Some(self.terrain.elevations()[(position.x, position.y)])
        } else {
            None
        }
    }

    pub fn get_rise(&self, edge: &Edge) -> Option<f32> {
        match (
            self.get_elevation(edge.from()),
            self.get_elevation(edge.to()),
        ) {
            (Some(from), Some(to)) => Some(to - from),
            _ => None,
        }
    }

    pub fn get_lowest_corner(&self, position: &V2<usize>) -> f32 {
        self.get_corners(&position)
            .iter()
            .flat_map(|corner| self.get_elevation(corner))
            .min_by(float_ordering)
            .unwrap()
    }

    #[allow(dead_code)]
    pub fn get_highest_corner(&self, position: &V2<usize>) -> f32 {
        self.get_corners(&position)
            .iter()
            .flat_map(|corner| self.get_elevation(corner))
            .max_by(float_ordering)
            .unwrap()
    }

    pub fn get_max_abs_rise(&self, position: &V2<usize>) -> f32 {
        self.get_border(&position)
            .iter()
            .flat_map(|edge| self.get_rise(edge))
            .map(|rise| rise.abs())
            .max_by(float_ordering)
            .unwrap()
    }

    pub fn expand_position(&self, position: &V2<usize>) -> Vec<V2<usize>> {
        let mut out = vec![];
        let fx = if position.x == 0 { 0 } else { position.x - 1 };
        let fy = if position.y == 0 { 0 } else { position.y - 1 };
        for x in fx..position.x + 2 {
            for y in fy..position.y + 2 {
                let position = v2(x, y);
                if self.in_bounds(&position) {
                    out.push(position);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[rustfmt::skip]
    fn world() -> World {
        World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 2.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            vec![
                Node::new(v2(1, 0), 0.1, 0.0),
                Node::new(v2(1, 1), 0.2, 0.0),
                Node::new(v2(1, 2), 0.3, 0.0),
                Node::new(v2(1, 2), 0.0, 0.3),
                Node::new(v2(2, 2), 0.0, 0.4),
            ],
            vec![
                Edge::new(v2(1, 0), v2(1, 1)),
                Edge::new(v2(1, 1), v2(1, 2)),
                Edge::new(v2(1, 2), v2(2, 2)),
            ],
            0.5
        )
    }

    #[test]
    fn test_terrain() {
        let terrain = world().terrain;

        assert_eq!(terrain.get_node(v2(1, 0)), &Node::new(v2(1, 0), 0.1, 0.0));
        assert_eq!(terrain.get_node(v2(1, 1)), &Node::new(v2(1, 1), 0.2, 0.0));
        assert_eq!(terrain.get_node(v2(1, 2)), &Node::new(v2(1, 2), 0.3, 0.3));
        assert_eq!(terrain.get_node(v2(2, 2)), &Node::new(v2(2, 2), 0.0, 0.4));
        assert!(terrain.is_edge(&Edge::new(v2(1, 0), v2(1, 1))));
        assert!(terrain.is_edge(&Edge::new(v2(1, 1), v2(1, 2))));
        assert!(terrain.is_edge(&Edge::new(v2(1, 2), v2(2, 2))));
    }

    #[rustfmt::skip]
    #[test]
    fn test_add_and_clear_road() {
        let mut world = world();

        let before_widths = M::from_vec(3, 3, vec![
            0.0, 0.1, 0.0,
            0.0, 0.2, 0.0,
            0.0, 0.3, 0.0,
        ]);
        let before_heights = M::from_vec(3, 3, vec![
            0.0, 0.0, 0.0,
            0.0, 0.0, 0.0,
            0.0, 0.3, 0.4,
        ]);
       
        for x in 0..3 {
            for y in 0..3 {
                assert_eq!(
                    world.terrain.get_node(v2(x, y)),
                    &Node::new(
                        v2(x, y), 
                        before_widths[(x, y)], 
                        before_heights[(x, y)]
                    ),
                );
            }
        }
        assert!(!world.terrain.is_edge(&Edge::new(v2(0, 0), v2(0, 1))));
        assert!(!world.terrain.is_edge(&Edge::new(v2(0, 1), v2(1, 1))));

        world.add_road(&Edge::new(v2(0, 0), v2(0, 1)));
        world.toggle_road(&Edge::new(v2(0, 1), v2(1, 1)));

        let after_widths = M::from_vec(3, 3, vec![
            World::ROAD_WIDTH, 0.1, 0.0,
            World::ROAD_WIDTH, 0.2, 0.0,
            0.0, 0.3, 0.0,
        ]);
        let after_heights = M::from_vec(3, 3, vec![
            0.0, 0.0, 0.0,
            World::ROAD_WIDTH, World::ROAD_WIDTH, 0.0,
            0.0, 0.3, 0.4,
        ]);

        for x in 0..3 {
            for y in 0..3 {
                assert_eq!(
                    world.terrain.get_node(v2(x, y)),
                    &Node::new(
                        v2(x, y),
                        after_widths[(x, y)],
                        after_heights[(x, y)]
                    ),
                );
            }
        }

        assert!(world.terrain.is_edge(&Edge::new(v2(0, 0), v2(0, 1))));
        assert!(world.terrain.is_edge(&Edge::new(v2(0, 1), v2(1, 1))));

        world.clear_road(&Edge::new(v2(0, 0), v2(0, 1)));
        world.toggle_road(&Edge::new(v2(0, 1), v2(1, 1)));

        for x in 0..3 {
            for y in 0..3 {
                assert_eq!(
                    world.terrain.get_node(v2(x, y)),
                     &Node::new(
                        v2(x, y), 
                        before_widths[(x, y)], 
                        before_heights[(x, y)]
                    ),
                );
            }
        }

        assert!(!world.terrain.is_edge(&Edge::new(v2(0, 0), v2(0, 1))));
        assert!(!world.terrain.is_edge(&Edge::new(v2(0, 1), v2(1, 1))));
    }

    #[test]
    fn test_snap() {
        assert_eq!(
            world().snap(WorldCoord::new(0.3, 1.7, 1.2)),
            WorldCoord::new(0.0, 2.0, 1.0)
        );
    }

    #[test]
    fn test_snap_middle() {
        assert_eq!(
            world().snap_middle(WorldCoord::new(0.3, 1.7, 1.2)),
            WorldCoord::new(0.5, 1.5, 2.0)
        );
    }

    #[test]
    fn test_get_corners() {
        assert_eq!(
            world().get_corners(&v2(0, 0)),
            [v2(0, 0), v2(1, 0), v2(1, 1), v2(0, 1)]
        );
    }

    #[test]
    fn test_get_border() {
        assert_eq!(
            world().get_border(&v2(0, 0)),
            vec![
                Edge::new(v2(0, 0), v2(1, 0)),
                Edge::new(v2(1, 0), v2(1, 1)),
                Edge::new(v2(1, 1), v2(0, 1)),
                Edge::new(v2(0, 1), v2(0, 0)),
            ]
        );
    }

    #[test]
    fn test_in_bounds() {
        assert!(world().in_bounds(&v2(1, 1)));
        assert!(!world().in_bounds(&v2(3, 1)));
        assert!(!world().in_bounds(&v2(1, 3)));
        assert!(!world().in_bounds(&v2(3, 3)));
    }

    #[test]
    fn test_get_elevation() {
        assert_eq!(world().get_elevation(&v2(1, 1)).unwrap(), 2.0);
    }

    #[test]
    fn test_get_rise() {
        assert_eq!(
            world().get_rise(&Edge::new(v2(1, 0), v2(1, 1))).unwrap(),
            1.0
        );
        assert_eq!(
            world().get_rise(&Edge::new(v2(1, 1), v2(2, 1))).unwrap(),
            -1.0
        );
        assert_eq!(
            world().get_rise(&Edge::new(v2(0, 0), v2(1, 0))).unwrap(),
            0.0
        );
    }

    #[test]
    fn test_get_lowest_corner() {
        assert_eq!(world().get_lowest_corner(&v2(0, 0)), 1.0);
    }

    #[test]
    fn test_get_highest_corner() {
        assert_eq!(world().get_highest_corner(&v2(0, 0)), 2.0);
    }

    #[test]
    fn test_get_max_abs_rise() {
        assert_eq!(world().get_max_abs_rise(&v2(0, 0)), 1.0);
    }

    #[test]
    fn test_expand() {
        let actual = world().expand_position(&v2(1, 1));
        assert_eq!(actual.len(), 9);
        assert!(actual.contains(&v2(0, 0)));
        assert!(actual.contains(&v2(1, 0)));
        assert!(actual.contains(&v2(2, 0)));
        assert!(actual.contains(&v2(0, 1)));
        assert!(actual.contains(&v2(1, 1)));
        assert!(actual.contains(&v2(1, 1)));
        assert!(actual.contains(&v2(0, 2)));
        assert!(actual.contains(&v2(1, 2)));
        assert!(actual.contains(&v2(2, 2)));
    }

    #[test]
    fn test_expand_top_left_corner() {
        let actual = world().expand_position(&v2(0, 0));
        assert_eq!(actual.len(), 4);
        assert!(actual.contains(&v2(0, 0)));
        assert!(actual.contains(&v2(1, 0)));
        assert!(actual.contains(&v2(0, 1)));
        assert!(actual.contains(&v2(1, 1)));
    }

    #[test]
    fn test_expand_bottom_right_corner() {
        let actual = world().expand_position(&v2(2, 2));
        assert_eq!(actual.len(), 4);
        assert!(actual.contains(&v2(2, 2)));
        assert!(actual.contains(&v2(2, 1)));
        assert!(actual.contains(&v2(1, 2)));
        assert!(actual.contains(&v2(1, 1)));
    }
}
