use cell_traits::*;
use commons::*;

pub struct TerrainGeometry<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    terrain: &'a Grid<T>,
}

impl<'a, T> TerrainGeometry<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    pub fn of(terrain: &'a Grid<T>) -> TerrainGeometry<'a, T> {
        TerrainGeometry { terrain }
    }

    fn get_vertex(&self, index: V2<usize>) -> Option<V3<f32>> {
        self.terrain
            .get_cell(&v2(index.x / 2, index.y / 2))
            .map(|cell| {
                let p = cell.get_float_position();
                let w = cell.junction().width();
                let h = cell.junction().height();
                let z = cell.elevation();
                match (index.x % 2, index.y % 2) {
                    (0, 0) => v3(p.x - w, p.y - h, z),
                    (1, 0) => v3(p.x + w, p.y - h, z),
                    (0, 1) => v3(p.x - w, p.y + h, z),
                    (1, 1) => v3(p.x + w, p.y + h, z),
                    (_, _) => panic!(
                        "Not expected to happen - {} % 2 or {} % 2 is not in range 0..1",
                        index.x, index.y
                    ),
                }
            })
    }

    fn get_border(&self, index: V2<usize>, include_invisible: bool) -> Vec<V3<f32>> {
        let offsets: [V2<usize>; 4] = [v2(0, 0), v2(1, 0), v2(1, 1), v2(0, 1)];

        let mut out = vec![];

        for o in 0..4 {
            let focus_index = index + offsets[o];

            if !include_invisible
                && !self
                    .terrain
                    .get_cell(&(focus_index / 2))
                    .unwrap()
                    .is_visible()
            {
                continue;
            }

            let next_index = index + offsets[(o + 1) % 4];

            let focus_position = self.get_vertex(v2(focus_index.x, focus_index.y));
            let next_position = self.get_vertex(v2(next_index.x, next_index.y));

            if let (Some(focus_position), Some(next_position)) = (focus_position, next_position) {
                if focus_position != next_position {
                    out.push(focus_position);
                }
            }
        }

        out
    }

    pub fn get_border_for_tile(
        &self,
        position: &V2<usize>,
        include_invisible: bool,
    ) -> Vec<V3<f32>> {
        self.get_border(get_index_for_tile(position), include_invisible)
    }

    fn get_triangles(&self, index: V2<usize>) -> Vec<[V3<f32>; 3]> {
        let border = self.get_border(index, false);

        if border.len() == 4 {
            vec![
                [border[0], border[3], border[2]],
                [border[0], border[2], border[1]],
            ]
        } else if border.len() == 3 {
            vec![[border[0], border[2], border[1]]]
        } else {
            vec![]
        }
    }

    pub fn get_triangles_for_node(&self, position: &V2<usize>) -> Vec<[V3<f32>; 3]> {
        self.get_triangles(get_index_for_node(position))
    }

    pub fn get_triangles_for_horizontal_edge(&self, position: &V2<usize>) -> Vec<[V3<f32>; 3]> {
        self.get_triangles(get_index_for_horizontal_edge(position))
    }

    pub fn get_triangles_for_vertical_edge(&self, position: &V2<usize>) -> Vec<[V3<f32>; 3]> {
        self.get_triangles(get_index_for_vertical_edge(position))
    }

    fn get_adjacent_non_edges_for_tile(&self, position: &V2<usize>) -> Vec<V2<usize>> {
        let index = get_index_for_tile(position);
        let cell = self.terrain.get_cell(position).unwrap();
        let opposite = self
            .terrain
            .get_cell(&v2(position.x + 1, position.y + 1))
            .unwrap();
        let mut out = vec![];
        if !cell.junction().horizontal.from {
            out.push(v2(index.x, index.y - 1))
        };
        if !cell.junction().vertical.from {
            out.push(v2(index.x - 1, index.y))
        };
        if !opposite.junction().horizontal.to {
            out.push(v2(index.x, index.y + 1))
        };
        if !opposite.junction().vertical.to {
            out.push(v2(index.x + 1, index.y))
        };
        out
    }

    pub fn get_triangles_for_tile(&self, position: &V2<usize>) -> Vec<[V3<f32>; 3]> {
        let index = get_index_for_tile(position);
        let mut out = vec![];
        out.append(&mut self.get_triangles(index));

        for adjacent in self.get_adjacent_non_edges_for_tile(position) {
            let triangles = self.get_triangles(adjacent);
            if triangles.len() == 1 || triangles.len() == 2 {
                for mut triangle in triangles {
                    for p in triangle.iter_mut() {
                        *p = clip_to_tile(*p, &position);
                    }
                    out.push(triangle);
                }
            }
        }

        out
    }
}

fn get_index_for_node(position: &V2<usize>) -> V2<usize> {
    V2::new(position.x * 2, position.y * 2)
}

fn get_index_for_horizontal_edge(position: &V2<usize>) -> V2<usize> {
    V2::new(position.x * 2 + 1, position.y * 2)
}

fn get_index_for_vertical_edge(position: &V2<usize>) -> V2<usize> {
    V2::new(position.x * 2, position.y * 2 + 1)
}

fn get_index_for_tile(position: &V2<usize>) -> V2<usize> {
    V2::new((position.x * 2) + 1, (position.y * 2) + 1)
}

fn clip_to_tile(mut point: V3<f32>, tile_coordinate: &V2<usize>) -> V3<f32> {
    let x = tile_coordinate.x as f32;
    let y = tile_coordinate.y as f32;
    point.x = point.x.max(x).min(x + 1.0);
    point.y = point.y.max(y).min(y + 1.0);

    point
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::junction::*;

    #[derive(Clone, Copy, PartialEq, Debug)]
    pub struct TestCell {
        pub position: V2<usize>,
        pub elevation: f32,
        pub visible: bool,
        pub junction: Junction,
    }

    impl TestCell {
        fn new(position: V2<usize>) -> TestCell {
            TestCell {
                position,
                elevation: 0.0,
                visible: true,
                junction: Junction::default(),
            }
        }
    }

    impl WithPosition for TestCell {
        fn position(&self) -> V2<usize> {
            self.position
        }
    }

    impl WithElevation for TestCell {
        fn elevation(&self) -> f32 {
            self.elevation
        }
    }

    impl WithVisibility for TestCell {
        fn is_visible(&self) -> bool {
            self.visible
        }
    }

    impl WithJunction for TestCell {
        fn junction(&self) -> Junction {
            self.junction
        }
    }

    #[rustfmt::skip]
    fn terrain() -> M<TestCell> {

        let mut terrain = M::from_fn(3, 3, |x, y| TestCell::new(v2(x, y)));
        terrain[(1, 1)].elevation = 4.0;
        terrain[(2, 1)].elevation = 3.0;
        terrain[(1, 2)].elevation = 2.0;
        terrain[(2, 2)].elevation = 1.0;
        terrain[(1, 1)].junction.horizontal.width = 0.5;
        terrain[(1, 1)].junction.vertical.width = 0.5;
        terrain[(2, 1)].junction.horizontal.width = 0.1;
        terrain[(2, 1)].junction.vertical.width = 0.4;
        terrain[(1, 2)].junction.horizontal.width = 0.4;
        terrain[(1, 2)].junction.vertical.width = 0.1;

        terrain[(1, 1)].junction.horizontal.from = true;
        terrain[(2, 1)].junction.horizontal.to = true;
        terrain[(2, 1)].junction.vertical.from = true;
        terrain[(2, 2)].junction.vertical.to = true;
        terrain[(1, 2)].junction.horizontal.from = true;
        terrain[(2, 2)].junction.horizontal.to = true;

        terrain.apply(&|cell| TestCell{visible: true, ..cell});

        terrain
    }

    #[test]
    fn test_get_terrain() {
        let mut expected = M::from_element(6, 6, v3(0.0, 0.0, 0.0));

        for x in 0..5 {
            for y in 0..5 {
                expected[(x, y)] = v3((x / 2) as f32, (y / 2) as f32, 0.0);
            }
        }

        expected[(2, 2)] = v3(0.5, 0.5, 4.0);
        expected[(3, 2)] = v3(1.5, 0.5, 4.0);
        expected[(2, 3)] = v3(0.5, 1.5, 4.0);
        expected[(3, 3)] = v3(1.5, 1.5, 4.0);

        expected[(4, 2)] = v3(1.6, 0.9, 3.0);
        expected[(5, 2)] = v3(2.4, 0.9, 3.0);
        expected[(4, 3)] = v3(1.6, 1.1, 3.0);
        expected[(5, 3)] = v3(2.4, 1.1, 3.0);

        expected[(2, 4)] = v3(0.9, 1.6, 2.0);
        expected[(3, 4)] = v3(1.1, 1.6, 2.0);
        expected[(2, 5)] = v3(0.9, 2.4, 2.0);
        expected[(3, 5)] = v3(1.1, 2.4, 2.0);

        expected[(4, 4)] = v3(2.0, 2.0, 1.0);
        expected[(5, 4)] = v3(2.0, 2.0, 1.0);
        expected[(4, 5)] = v3(2.0, 2.0, 1.0);
        expected[(5, 5)] = v3(2.0, 2.0, 1.0);

        let terrain = terrain();
        let geometry = TerrainGeometry::of(&terrain);

        for x in 0..5 {
            for y in 0..5 {
                assert_eq!(geometry.get_vertex(v2(x, y)), Some(expected[(x, y)]));
            }
        }
    }

    #[test]
    fn test_get_border_square() {
        let actual = TerrainGeometry::of(&terrain()).get_border(v2(2, 2), true);

        assert_eq!(
            actual,
            vec![
                v3(0.5, 0.5, 4.0),
                v3(1.5, 0.5, 4.0),
                v3(1.5, 1.5, 4.0),
                v3(0.5, 1.5, 4.0),
            ]
        );
    }

    #[test]
    fn test_get_border_triangle() {
        let actual = TerrainGeometry::of(&terrain()).get_border(v2(2, 1), true);

        assert_eq!(
            actual,
            vec![v3(1.0, 0.0, 0.0), v3(1.5, 0.5, 4.0), v3(0.5, 0.5, 4.0),]
        );
    }

    #[test]
    fn test_get_border_line() {
        let actual = TerrainGeometry::of(&terrain()).get_border(v2(1, 0), true);

        assert_eq!(actual, vec![v3(0.0, 0.0, 0.0), v3(1.0, 0.0, 0.0),]);
    }

    #[test]
    fn test_get_border_empty() {
        let actual = TerrainGeometry::of(&terrain()).get_border(v2(0, 0), true);

        assert_eq!(actual, vec![]);
    }

    #[test]
    fn test_get_border_with_invisible_vertices() {
        let mut terrain = terrain();

        assert_eq!(
            TerrainGeometry::of(&terrain).get_border(v2(3, 2), false),
            vec![
                v3(1.5, 0.5, 4.0),
                v3(1.6, 0.9, 3.0),
                v3(1.6, 1.1, 3.0),
                v3(1.5, 1.5, 4.0),
            ]
        );

        terrain[(1, 1)].visible = false;

        assert_eq!(
            TerrainGeometry::of(&terrain).get_border(v2(3, 2), false),
            vec![v3(1.6, 0.9, 3.0), v3(1.6, 1.1, 3.0),]
        );

        terrain[(2, 1)].visible = false;

        assert_eq!(
            TerrainGeometry::of(&terrain).get_border(v2(3, 2), false),
            vec![]
        );
    }

    #[test]
    fn test_get_triangles_square() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles(v2(2, 2));

        assert_eq!(
            actual,
            vec![
                [v3(0.5, 0.5, 4.0), v3(0.5, 1.5, 4.0), v3(1.5, 1.5, 4.0)],
                [v3(0.5, 0.5, 4.0), v3(1.5, 1.5, 4.0), v3(1.5, 0.5, 4.0)],
            ]
        );
    }

    #[test]
    fn test_get_triangles_triangle() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles(v2(2, 1));

        assert_eq!(
            actual,
            vec![[v3(1.0, 0.0, 0.0), v3(0.5, 0.5, 4.0), v3(1.5, 0.5, 4.0)],]
        );
    }

    #[test]
    fn test_get_triangles_line() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles(v2(1, 0));
        let expected: Vec<[V3<f32>; 3]> = vec![];

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_triangles_point() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles(v2(0, 0));
        let expected: Vec<[V3<f32>; 3]> = vec![];

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_index_for_node() {
        let mut actual = vec![];
        for y in 0..3 {
            for x in 0..3 {
                actual.push(get_index_for_node(&v2(x, y)));
            }
        }
        let expected = vec![
            v2(0, 0),
            v2(2, 0),
            v2(4, 0),
            v2(0, 2),
            v2(2, 2),
            v2(4, 2),
            v2(0, 4),
            v2(2, 4),
            v2(4, 4),
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_index_for_tile() {
        let mut actual = vec![];
        for y in 0..2 {
            for x in 0..2 {
                actual.push(get_index_for_tile(&v2(x, y)));
            }
        }
        let expected = vec![v2(1, 1), v2(3, 1), v2(1, 3), v2(3, 3)];

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_clip_to_tile() {
        let actual = clip_to_tile(v3(9.5, 10.5, 12.0), &v2(10, 10));
        assert_eq!(actual, v3(10.0, 10.5, 12.0));

        let actual = clip_to_tile(v3(11.5, 10.5, 12.0), &v2(10, 10));
        assert_eq!(actual, v3(11.0, 10.5, 12.0));

        let actual = clip_to_tile(v3(10.5, 9.5, 12.0), &v2(10, 10));
        assert_eq!(actual, v3(10.5, 10.0, 12.0));

        let actual = clip_to_tile(v3(10.5, 11.5, 12.0), &v2(10, 10));
        assert_eq!(actual, v3(10.5, 11.0, 12.0));
    }

    #[test]
    fn test_get_triangles_for_tile_a() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles_for_tile(&v2(1, 0));

        assert!(actual.contains(&[v3(1.0, 0.0, 0.0), v3(1.5, 0.5, 4.0), v3(1.6, 0.9, 3.0)]));
        assert!(actual.contains(&[v3(1.0, 0.0, 0.0), v3(1.6, 0.9, 3.0), v3(2.0, 0.0, 0.0)]));
        assert!(actual.contains(&[v3(1.0, 0.0, 0.0), v3(1.0, 0.5, 4.0), v3(1.5, 0.5, 4.0)]));
        assert!(actual.contains(&[v3(2.0, 0.0, 0.0), v3(1.6, 0.9, 3.0), v3(2.0, 0.9, 3.0)]));
        assert_eq!(actual.len(), 4);
    }

    #[test]
    fn test_get_triangles_for_tile_b() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles_for_tile(&v2(1, 1));

        assert!(actual.contains(&[v3(1.5, 1.5, 4.0), v3(1.1, 1.6, 2.0), v3(2.0, 2.0, 1.0)]));
        assert!(actual.contains(&[v3(1.5, 1.5, 4.0), v3(2.0, 2.0, 1.0), v3(1.6, 1.1, 3.0)]));
        assert!(actual.contains(&[v3(1.0, 1.5, 4.0), v3(1.0, 1.6, 2.0), v3(1.1, 1.6, 2.0)]));
        assert!(actual.contains(&[v3(1.0, 1.5, 4.0), v3(1.1, 1.6, 2.0), v3(1.5, 1.5, 4.0)]));
        assert_eq!(actual.len(), 4);
    }

    #[test]
    fn test_get_triangles_for_tile_with_invisible_vertex() {
        let mut terrain = terrain();
        terrain[(2, 2)].visible = false;

        let actual = TerrainGeometry::of(&terrain).get_triangles_for_tile(&v2(1, 1));

        assert!(actual.contains(&[v3(1.5, 1.5, 4.0), v3(1.1, 1.6, 2.0), v3(1.6, 1.1, 3.0)]));
        assert!(actual.contains(&[v3(1.0, 1.5, 4.0), v3(1.0, 1.6, 2.0), v3(1.1, 1.6, 2.0)]));
        assert!(actual.contains(&[v3(1.0, 1.5, 4.0), v3(1.1, 1.6, 2.0), v3(1.5, 1.5, 4.0)]));
        assert_eq!(actual.len(), 3);
    }

}
