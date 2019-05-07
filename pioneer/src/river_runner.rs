use downhill_map::DownhillMap;
use downhill_map::DIRECTIONS;
use flow_map::FlowMap;
use isometric::terrain::{Edge, Node};
use mesh::Mesh;
use rand::prelude::*;
use scale::Scale;
use single_downhill_map::{RandomDownhillMap, SingleDownhillMap};

pub fn get_junctions_and_rivers<R: Rng>(
    mesh: &Mesh,
    threshold: u32,
    sea_level: f64,
    flow_to_width: (f64, f64),
    rng: &mut Box<R>,
) -> (Vec<Node>, Vec<Edge>) {
    let downhill_map = DownhillMap::new(&mesh);
    let random_downhill_map: Box<SingleDownhillMap> =
        Box::new(RandomDownhillMap::new(&downhill_map, rng));

    get_junctions_and_rivers_from_downhill_map(
        &mesh,
        threshold,
        sea_level,
        flow_to_width,
        &random_downhill_map,
    )
}

fn get_junctions_and_rivers_from_downhill_map(
    mesh: &Mesh,
    threshold: u32,
    sea_level: f64,
    flow_to_width: (f64, f64),
    downhill_map: &Box<SingleDownhillMap>,
) -> (Vec<Node>, Vec<Edge>) {
    let flow_map = FlowMap::from(&mesh, &downhill_map);
    get_junctions_and_rivers_from_flow_map(
        &mesh,
        threshold,
        sea_level,
        flow_to_width,
        &downhill_map,
        &flow_map,
    )
}

fn get_neighbour(
    position: na::Vector2<usize>,
    mesh: &Mesh,
    downhill_map: &Box<SingleDownhillMap>,
) -> Option<na::Vector2<usize>> {
    let direction = DIRECTIONS[downhill_map.get_direction(position.x as i32, position.y as i32)];
    let nx = (position.x as i32) + direction.0;
    let ny = (position.y as i32) + direction.1;
    if mesh.in_bounds(nx, ny) {
        Some(na::Vector2::new(nx as usize, ny as usize))
    } else {
        None
    }
}

fn get_max_flow_over_sea_level(mesh: &Mesh, sea_level: f64, flow_map: &FlowMap) -> u32 {
    let mut out = 0;
    for x in 0..mesh.get_width() {
        for y in 0..mesh.get_width() {
            if mesh.get_z(x, y) >= sea_level {
                out = out.max(flow_map.get_flow(x, y));
            }
        }
    }
    out
}

fn get_junctions_and_rivers_from_flow_map(
    mesh: &Mesh,
    threshold: u32,
    sea_level: f64,
    flow_to_width: (f64, f64),
    downhill_map: &Box<SingleDownhillMap>,
    flow_map: &FlowMap,
) -> (Vec<Node>, Vec<Edge>) {
    let mut junctions = vec![];
    let mut rivers = vec![];

    let max_flow_over_sea_level = get_max_flow_over_sea_level(mesh, sea_level, flow_map) as f64;
    let flow_scale = Scale::new((threshold as f64, max_flow_over_sea_level), flow_to_width);

    for x in 0..mesh.get_width() {
        for y in 0..mesh.get_width() {
            let flow = flow_map.get_flow(x, y);
            if flow >= threshold && mesh.get_z(x, y) >= sea_level {
                let position = na::Vector2::new(x as usize, y as usize);
                if let Some(neighbour) = get_neighbour(position, mesh, downhill_map) {
                    let neighbour_flow = flow_map.get_flow(neighbour.x as i32, neighbour.y as i32);
                    let from_width = flow_scale.scale(flow as f64) as f32;
                    let to_width = flow_scale.scale(neighbour_flow as f64) as f32;
                    if position.x == neighbour.x {
                        junctions.push(Node::new(position, from_width, 0.0));
                        junctions.push(Node::new(neighbour, to_width, 0.0));
                    } else {
                        junctions.push(Node::new(position, 0.0, from_width));
                        junctions.push(Node::new(neighbour, 0.0, to_width));
                    }

                    rivers.push(Edge::new(position, neighbour));
                }
            }
        }
    }

    (junctions, rivers)
}

#[cfg(test)]
mod tests {

    use super::*;
    use single_downhill_map::MockDownhillMap;

    fn mesh() -> Mesh {
        let mut mesh = Mesh::new(4, 0.0);
        let z = na::DMatrix::from_row_slice(
            4,
            4,
            &[
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ],
        );
        mesh.set_z_vector(z);
        mesh
    }

    fn downhill_map() -> Box<SingleDownhillMap> {
        let downhill_map = vec![
            vec![3, 3, 3, 3],
            vec![3, 3, 3, 3],
            vec![0, 0, 0, 0],
            vec![0, 0, 0, 0],
        ];
        let downhill_map = MockDownhillMap::new(downhill_map);
        Box::new(downhill_map)
    }

    fn flow_map() -> FlowMap {
        let mut flow_map = FlowMap::new(4);
        flow_map.set_flow(na::DMatrix::from_row_slice(
            4,
            4,
            &[1, 2, 5, 7, 3, 7, 9, 12, 2, 2, 2, 2, 1, 1, 1, 1],
        ));
        flow_map
    }

    #[test]
    fn test_get_downhill() {
        let position = na::Vector2::new(1, 2);
        assert_eq!(
            get_neighbour(position, &mesh(), &downhill_map()),
            Some(na::Vector2::new(1, 3))
        );
    }

    #[test]
    fn test_get_downhill_out_of_bounds() {
        let position = na::Vector2::new(1, 3);
        assert_eq!(get_neighbour(position, &mesh(), &downhill_map()), None);
    }

    #[test]
    fn test_get_max_flow_over_sea_level() {
        assert_eq!(get_max_flow_over_sea_level(&mesh(), 0.5, &flow_map()), 7);
    }

    #[test]
    fn test_get_junctions_and_rivers_from_flow_map() {
        let (junctions, rivers) = get_junctions_and_rivers_from_flow_map(
            &mesh(),
            3,
            0.5,
            (0.0, 1.0),
            &downhill_map(),
            &flow_map(),
        );

        assert!(junctions.contains(&Node::new(na::Vector2::new(1, 0), 0.0, 0.0)));
        assert!(junctions.contains(&Node::new(na::Vector2::new(1, 1), 1.0, 0.0)));
        assert!(rivers.contains(&Edge::new(na::Vector2::new(1, 0), na::Vector2::new(1, 1))));
        assert!(junctions.contains(&Node::new(na::Vector2::new(1, 1), 1.0, 0.0)));
        assert!(junctions.contains(&Node::new(na::Vector2::new(1, 2), 1.5, 0.0)));
        assert!(rivers.contains(&Edge::new(na::Vector2::new(1, 1), na::Vector2::new(1, 2))));
        assert!(junctions.contains(&Node::new(na::Vector2::new(0, 2), 0.5, 0.0)));
        assert!(junctions.contains(&Node::new(na::Vector2::new(0, 3), 1.0, 0.0)));
        assert!(rivers.contains(&Edge::new(na::Vector2::new(0, 2), na::Vector2::new(0, 3))));
        assert_eq!(rivers.len(), 3);
        assert_eq!(junctions.len(), 6);
    }
}
