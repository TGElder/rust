use downhill_map::DownhillMap;
use flow_map::FlowMap;
use mesh::Mesh;
use rand::prelude::*;
use single_downhill_map::{RandomDownhillMap, SingleDownhillMap};

pub struct Erosion {}

impl Erosion {
    pub fn erode<R: Rng>(
        mut mesh: Mesh,
        rng: &mut R,
        threshold: u32,
        samples: usize,
        erosion: f64,
    ) -> Mesh {
        let downhill_map = DownhillMap::new(&mesh);
        let mut eroded = vec![vec![false; mesh.get_width() as usize]; mesh.get_width() as usize];
        for _ in 0..samples {
            let random_downhill_map = RandomDownhillMap::new(&downhill_map, rng);
            let random_downhill_map: Box<SingleDownhillMap> = Box::new(random_downhill_map);
            let flow_map = FlowMap::from(&mesh, &random_downhill_map);
            for x in 0..mesh.get_width() {
                for y in 0..mesh.get_width() {
                    if !eroded[x as usize][y as usize] {
                        let flow = flow_map.get_flow(x, y);
                        if flow > 1 && flow > threshold {
                            let after = mesh.get_z(x, y) * erosion;
                            mesh.set_z(x, y, after);
                            eroded[x as usize][y as usize] = true;
                        }
                    }
                }
            }
        }
        mesh
    }
}
