use cell_traits::*;
use commons::grid::Grid;
use commons::*;

pub struct TerrainGeometry<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    terrain: &'a dyn Grid<T>,
}

impl<'a, T> TerrainGeometry<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    pub fn of(terrain: &'a dyn Grid<T>) -> TerrainGeometry<'a, T> {
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

    pub fn get_original_border_for_tile(&self, position: &V2<usize>) -> Vec<V3<f32>> {
        let offsets: [V2<usize>; 4] = [v2(0, 0), v2(1, 0), v2(1, 1), v2(0, 1)];
        offsets
            .iter()
            .map(|o| {
                v3(
                    (position.x + o.x) as f32,
                    (position.y + o.y) as f32,
                    self.terrain
                        .get_cell_unsafe(&v2(position.x + o.x, position.y + o.y))
                        .elevation(),
                )
            })
            .collect()
    }

    fn get_triangles(&self, index: V2<usize>) -> Vec<[V3<f32>; 3]> {
        let offsets: [V2<usize>; 4] = [v2(0, 0), v2(1, 0), v2(1, 1), v2(0, 1)];
        let positions = [
            index + offsets[0],
            index + offsets[1],
            index + offsets[2],
            index + offsets[3],
        ];
        let vertices = [
            self.get_vertex(positions[0]).unwrap(),
            self.get_vertex(positions[1]).unwrap(),
            self.get_vertex(positions[2]).unwrap(),
            self.get_vertex(positions[3]).unwrap(),
        ];
        let highest_index = get_index_of_highest_border_point(&vertices);

        // Divides square into triangles so highest corner is only in one triangle
        // This results in nice coastline if all but one corner is below sea level
        let triangle_indices = if highest_index == 0 || highest_index == 2 {
            [[0, 1, 3], [1, 2, 3]]
        } else {
            [[0, 2, 3], [0, 1, 2]]
        };

        triangle_indices
            .iter()
            .filter(|[a, b, c]| {
                self.triangle_is_visible(&[positions[*a], positions[*b], positions[*c]])
            })
            .map(|[a, b, c]| [vertices[*a], vertices[*b], vertices[*c]])
            .filter(|[a, b, c]| a != b && b != c && c != a)
            .collect()
    }

    fn triangle_is_visible(&self, triangle: &[V2<usize>; 3]) -> bool {
        triangle
            .iter()
            .all(|corner| self.terrain.get_cell_unsafe(&(corner / 2)).is_visible())
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
                        *p = clip_to_tile(*p, position);
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

fn get_index_of_highest_border_point(border: &[V3<f32>]) -> usize {
    border
        .iter()
        .enumerate()
        .max_by(|a, b| unsafe_ordering(&a.1.z, &b.1.z))
        .map(|(i, _)| i)
        .unwrap()
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
    fn test_get_original_border_for_tile() {
        let terrain = terrain();

        assert_eq!(
            TerrainGeometry::of(&terrain).get_original_border_for_tile(&v2(1, 1)),
            vec![
                v3(1.0, 1.0, 4.0),
                v3(2.0, 1.0, 3.0),
                v3(2.0, 2.0, 1.0),
                v3(1.0, 2.0, 2.0),
            ]
        );
    }

    #[test]
    fn test_get_triangles_square_highest_corner_top_left() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles(v2(3, 3));

        assert_eq!(
            actual,
            vec![
                [v3(1.5, 1.5, 4.0), v3(1.6, 1.1, 3.0), v3(1.1, 1.6, 2.0)],
                [v3(1.6, 1.1, 3.0), v3(2.0, 2.0, 1.0), v3(1.1, 1.6, 2.0)],
            ]
        );
    }

    #[test]
    fn test_get_triangles_square_highest_corner_top_right() {
        let mut terrain = terrain();
        terrain[(1, 1)].elevation = 1.0;

        let actual = TerrainGeometry::of(&terrain).get_triangles(v2(3, 3));

        assert_eq!(
            actual,
            vec![
                [v3(1.5, 1.5, 1.0), v3(2.0, 2.0, 1.0), v3(1.1, 1.6, 2.0)],
                [v3(1.5, 1.5, 1.0), v3(1.6, 1.1, 3.0), v3(2.0, 2.0, 1.0)],
            ]
        );
    }

    #[test]
    fn test_get_triangles_square_with_invisible_vertex() {
        let mut terrain = terrain();
        terrain[(1, 1)].visible = false;

        let actual = TerrainGeometry::of(&terrain).get_triangles(v2(3, 3));

        assert_eq!(
            actual,
            vec![[v3(1.6, 1.1, 3.0), v3(2.0, 2.0, 1.0), v3(1.1, 1.6, 2.0)],]
        );
    }

    #[test]
    fn test_get_triangles_triangle() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles(v2(2, 1));

        assert_eq!(
            actual,
            vec![[v3(1.0, 0.0, 0.0), v3(1.5, 0.5, 4.0), v3(0.5, 0.5, 4.0)],]
        );
    }

    #[test]
    fn test_get_triangles_line() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles(v2(1, 0));
        let expected: Vec<[V3<f32>; 3]> = vec![];

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_triangles_empty() {
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

        assert!(actual.contains(&[v3(1.0, 0.0, 0.0), v3(1.6, 0.9, 3.0), v3(1.5, 0.5, 4.0)]));
        assert!(actual.contains(&[v3(1.0, 0.0, 0.0), v3(2.0, 0.0, 0.0), v3(1.6, 0.9, 3.0)]));
        assert!(actual.contains(&[v3(1.0, 0.0, 0.0), v3(1.5, 0.5, 4.0), v3(1.0, 0.5, 4.0)]));
        assert!(actual.contains(&[v3(2.0, 0.0, 0.0), v3(2.0, 0.9, 3.0), v3(1.6, 0.9, 3.0)]));
        assert_eq!(actual.len(), 4);
    }

    #[test]
    fn test_get_triangles_for_tile_b() {
        let actual = TerrainGeometry::of(&terrain()).get_triangles_for_tile(&v2(1, 1));

        assert!(actual.contains(&[v3(1.5, 1.5, 4.0), v3(1.6, 1.1, 3.0), v3(1.1, 1.6, 2.0)]));
        assert!(actual.contains(&[v3(1.6, 1.1, 3.0), v3(2.0, 2.0, 1.0), v3(1.1, 1.6, 2.0)]));
        assert!(actual.contains(&[v3(1.0, 1.5, 4.0), v3(1.1, 1.6, 2.0), v3(1.0, 1.6, 2.0)]));
        assert!(actual.contains(&[v3(1.0, 1.5, 4.0), v3(1.5, 1.5, 4.0), v3(1.1, 1.6, 2.0)]));
        assert_eq!(actual.len(), 4);
    }

    #[test]
    fn test_get_triangles_for_tile_with_invisible_vertex() {
        let mut terrain = terrain();
        terrain[(2, 2)].visible = false;

        let actual = TerrainGeometry::of(&terrain).get_triangles_for_tile(&v2(1, 1));

        assert!(actual.contains(&[v3(1.5, 1.5, 4.0), v3(1.6, 1.1, 3.0), v3(1.1, 1.6, 2.0)]));
        assert!(actual.contains(&[v3(1.0, 1.5, 4.0), v3(1.1, 1.6, 2.0), v3(1.0, 1.6, 2.0)]));
        assert!(actual.contains(&[v3(1.0, 1.5, 4.0), v3(1.5, 1.5, 4.0), v3(1.1, 1.6, 2.0)]));
        assert_eq!(actual.len(), 3);
    }
}
