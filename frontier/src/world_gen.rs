use crate::world::World;
use pioneer::erosion::Erosion;
use pioneer::mesh::Mesh;
use pioneer::mesh_splitter::MeshSplitter;
use pioneer::rand::prelude::*;
use pioneer::river_runner::get_junctions_and_rivers;
use pioneer::scale::Scale;
use std::f64::MAX;

pub fn generate_world(size: usize, seed: u8) -> World {
    let mut mesh = Mesh::new(1, 0.0);
    mesh.set_z(0, 0, MAX);
    let mut rng = Box::new(SmallRng::from_seed([seed; 16]));

    println!("Generating world...");
    for i in 0..size {
        mesh = MeshSplitter::split(&mesh, &mut rng, (0.0, 0.75));
        if i < 9 {
            let threshold = i * 2;
            mesh = Erosion::erode(mesh, &mut rng, threshold as u32, 16);
        }
        println!("{}", size - i);
    }

    let max_height = (2.0 as f64).powf(size as f64) / 16.0;
    let sea_level = 0.5;
    let before_sea_level =
        Scale::new((0.0, max_height), (mesh.get_min_z(), mesh.get_max_z())).scale(sea_level);
    let (junctions, rivers) =
        get_junctions_and_rivers(&mesh, 256, before_sea_level, (0.01, 0.49), &mut rng);

    mesh = mesh.rescale(&Scale::new(
        (mesh.get_min_z(), mesh.get_max_z()),
        (0.0, max_height),
    ));
    let terrain = mesh.get_z_vector().map(|z| z as f32);

    World::new(terrain, junctions, rivers, sea_level as f32)
}
