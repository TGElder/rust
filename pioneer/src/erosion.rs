use commons::*;
use downhill_map::DownhillMap;
use flow_map::FlowMap;
use mesh::Mesh;
use rand::prelude::*;
use single_downhill_map::RandomDownhillMap;

pub struct Erosion {}

impl Erosion {
    pub fn erode<R: Rng>(
        mut mesh: Mesh,
        rng: &mut R,
        threshold: f64,
        samples: usize,
        erosion: f64,
    ) -> Mesh {
        let downhill_map = DownhillMap::new(&mesh);
        let width = mesh.get_width() as usize;
        let mut eroded = M::from_element(width, width, false);
        let rainfall = M::from_element(width, width, 1.0);
        for _ in 0..samples {
            let random_downhill_map = RandomDownhillMap::new(&downhill_map, rng);
            let flow_map = FlowMap::from(&mesh, &random_downhill_map, &rainfall);
            for x in 0..mesh.get_width() {
                for y in 0..mesh.get_width() {
                    if !eroded[(x as usize, y as usize)] {
                        let flow = flow_map.get_flow(x, y);
                        if flow > 1.0 && flow > threshold {
                            let after = mesh.get_z(x, y) * erosion;
                            mesh.set_z(x, y, after);
                            eroded[(x as usize, y as usize)] = true;
                        }
                    }
                }
            }
        }
        mesh
    }
}
