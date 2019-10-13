use commons::edge::*;
use commons::junction::*;
use commons::*;
use downhill_map::DownhillMap;
use downhill_map::DIRECTIONS;
use flow_map::FlowMap;
use mesh::Mesh;
use rand::prelude::*;
use scale::Scale;
use single_downhill_map::{RandomDownhillMap, SingleDownhillMap};

pub fn get_river_cells<R: Rng>(
    mesh: &Mesh,
    threshold: f64,
    sea_level: f64,
    flow_to_width: (f64, f64),
    rainfall: &M<f64>,
    rng: &mut R,
) -> Vec<PositionJunction> {
    let downhill_map = DownhillMap::new(&mesh);
    let random_downhill_map = RandomDownhillMap::new(&downhill_map, rng);

    get_river_cells_from_downhill_map_and_rain_map(
        &mesh,
        threshold,
        sea_level,
        flow_to_width,
        &random_downhill_map,
        &rainfall,
    )
}

fn get_river_cells_from_downhill_map_and_rain_map(
    mesh: &Mesh,
    threshold: f64,
    sea_level: f64,
    flow_to_width: (f64, f64),
    downhill_map: &dyn SingleDownhillMap,
    rainfall: &M<f64>,
) -> Vec<PositionJunction> {
    let flow_map = FlowMap::from(&mesh, downhill_map, &rainfall);
    let junctions = get_junction_matrix_from_flow_map(
        &mesh,
        threshold,
        sea_level,
        flow_to_width,
        downhill_map,
        &flow_map,
    );
    get_river_cells_from_junction_matrix(junctions)
}

fn get_neighbour(
    position: na::Vector2<usize>,
    mesh: &Mesh,
    downhill_map: &dyn SingleDownhillMap,
) -> Option<na::Vector2<usize>> {
    let direction = DIRECTIONS[downhill_map.get_direction(position.x as i32, position.y as i32)];
    let nx = (position.x as i32) + direction.0;
    let ny = (position.y as i32) + direction.1;
    if mesh.in_bounds(nx, ny) {
        Some(v2(nx as usize, ny as usize))
    } else {
        None
    }
}

fn get_max_flow_over_sea_level(mesh: &Mesh, sea_level: f64, flow_map: &FlowMap) -> f64 {
    let mut out: f64 = 0.0;
    for x in 0..mesh.get_width() {
        for y in 0..mesh.get_width() {
            if mesh.get_z(x, y) >= sea_level {
                out = out.max(flow_map.get_flow(x, y));
            }
        }
    }
    out
}

fn get_junction_matrix_from_flow_map(
    mesh: &Mesh,
    threshold: f64,
    sea_level: f64,
    flow_to_width: (f64, f64),
    downhill_map: &dyn SingleDownhillMap,
    flow_map: &FlowMap,
) -> M<Junction> {
    let width = mesh.get_width() as usize;
    let mut junctions = M::from_element(width, width, Junction::default());

    let max_flow_over_sea_level = get_max_flow_over_sea_level(mesh, sea_level, flow_map) as f64;
    let flow_scale = Scale::new((threshold as f64, max_flow_over_sea_level), flow_to_width);

    for x in 0..mesh.get_width() {
        for y in 0..mesh.get_width() {
            let flow = flow_map.get_flow(x, y);
            if flow >= threshold && mesh.get_z(x, y) >= sea_level {
                let position = v2(x as usize, y as usize);
                if let Some(neighbour) = get_neighbour(position, mesh, downhill_map) {
                    let neighbour_flow = flow_map.get_flow(neighbour.x as i32, neighbour.y as i32);
                    let position_width = flow_scale.scale(flow as f64) as f32;
                    let neighbour_width = flow_scale.scale(neighbour_flow as f64) as f32;
                    let edge = Edge::new(position, v2(neighbour.x, neighbour.y));
                    junctions
                        .mut_cell_unsafe(&position)
                        .junction_1d(edge.horizontal())
                        .width = position_width;
                    junctions
                        .mut_cell_unsafe(&neighbour)
                        .junction_1d(edge.horizontal())
                        .width = neighbour_width;
                    junctions
                        .mut_cell_unsafe(edge.from())
                        .junction_1d(edge.horizontal())
                        .from = true;
                    junctions
                        .mut_cell_unsafe(edge.to())
                        .junction_1d(edge.horizontal())
                        .to = true;
                }
            }
        }
    }

    junctions
}

fn get_river_cells_from_junction_matrix(junctions: M<Junction>) -> Vec<PositionJunction> {
    let (width, height) = junctions.shape();
    let default = Junction::default();
    let mut out = vec![];
    for x in 0..width {
        for y in 0..height {
            let junction = junctions[(x, y)];
            if junctions[(x, y)] != default {
                out.push(PositionJunction {
                    position: v2(x, y),
                    junction,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {

    use super::*;
    use single_downhill_map::MockDownhillMap;

    #[rustfmt::skip]
    fn mesh() -> Mesh {
        let mut mesh = Mesh::new(4, 0.0);
        let z = M::from_row_slice(
            4,
            4,
            &[
                1.0, 1.0, 1.0, 1.0,
                1.0, 1.0, 0.0, 0.0,
                1.0, 1.0, 1.0, 1.0,
                1.0, 1.0, 1.0, 1.0,
            ],
        );
        mesh.set_z_vector(z);
        mesh
    }

    #[rustfmt::skip]
    fn downhill_map() -> MockDownhillMap {
        let downhill_map = vec![
            vec![3, 3, 3, 3],
            vec![3, 3, 3, 3],
            vec![0, 0, 0, 0],
            vec![0, 0, 0, 0],
        ];
        MockDownhillMap::new(downhill_map)
    }

    #[rustfmt::skip]
    fn flow_map() -> FlowMap {
        let mut flow_map = FlowMap::new(4);
        flow_map.set_flow(M::from_row_slice(
            4,
            4,
            &[
                1.0, 2.0, 5.0, 7.0,
                3.0, 7.0, 9.0, 12.0,
                3.0, 2.0, 2.0, 2.0,
                1.0, 1.0, 1.0, 1.0
            ],
        ));
        flow_map
    }

    #[test]
    fn test_get_downhill() {
        let position = v2(1, 2);
        assert_eq!(
            get_neighbour(position, &mesh(), &downhill_map()),
            Some(v2(1, 3))
        );
    }

    #[test]
    fn test_get_downhill_out_of_bounds() {
        let position = v2(1, 3);
        assert_eq!(get_neighbour(position, &mesh(), &downhill_map()), None);
    }

    #[test]
    fn test_get_max_flow_over_sea_level() {
        assert!(get_max_flow_over_sea_level(&mesh(), 0.5, &flow_map()).almost(7.0));
    }

    #[test]
    fn test_get_junctions_and_rivers_from_flow_map() {
        let junctions = get_junction_matrix_from_flow_map(
            &mesh(),
            3.0,
            0.5,
            (0.0, 1.0),
            &downhill_map(),
            &flow_map(),
        );

        assert_eq!(
            junctions[(2, 0)],
            Junction {
                horizontal: Junction1D {
                    width: 0.0,
                    from: false,
                    to: true
                },
                vertical: Junction1D::default(),
            }
        );
        assert_eq!(
            junctions[(1, 0)],
            Junction {
                horizontal: Junction1D {
                    width: 0.0,
                    from: true,
                    to: false
                },
                vertical: Junction1D {
                    width: 0.0,
                    from: true,
                    to: false
                },
            }
        );
        assert_eq!(
            junctions[(1, 1)],
            Junction {
                horizontal: Junction1D::default(),
                vertical: Junction1D {
                    width: 1.0,
                    from: true,
                    to: true
                },
            }
        );

        assert_eq!(
            junctions[(1, 2)],
            Junction {
                horizontal: Junction1D::default(),
                vertical: Junction1D {
                    width: 1.5,
                    from: false,
                    to: true
                },
            }
        );

        assert_eq!(
            junctions[(0, 2)],
            Junction {
                horizontal: Junction1D::default(),
                vertical: Junction1D {
                    width: 0.5,
                    from: true,
                    to: false
                },
            }
        );

        assert_eq!(
            junctions[(0, 3)],
            Junction {
                horizontal: Junction1D::default(),
                vertical: Junction1D {
                    width: 1.0,
                    from: false,
                    to: true
                },
            }
        );

        assert_eq!(get_river_cells_from_junction_matrix(junctions).len(), 6);
    }
}
