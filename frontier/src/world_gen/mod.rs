mod temperature_gen;

use self::temperature_gen::*;
use crate::world::World;
use commons::scale::Scale;
use pioneer::erosion::Erosion;
use pioneer::mesh::Mesh;
use pioneer::mesh_splitter::MeshSplitter;
use pioneer::rand::prelude::*;
use pioneer::river_runner::*;
use std::f64::MAX;

pub fn rng(seed: u8) -> SmallRng {
    SmallRng::from_seed([seed; 16])
}

pub fn generate_world<T: Rng>(size: usize, rng: &mut T) -> World {
    let mut mesh = Mesh::new(1, 0.0);
    mesh.set_z(0, 0, MAX);

    println!("Generating world...");
    for i in 0..size {
        mesh = MeshSplitter::split(&mesh, rng, (0.0, 0.75));
        if i < (size - 1) {
            let threshold = i * 2;
            mesh = Erosion::erode(mesh, rng, threshold as u32, 16, 0.9);
        }
        println!("{}", size - i);
    }

    let max_height = 32.0;
    let sea_level = 0.5;
    let before_sea_level =
        Scale::new((0.0, max_height), (mesh.get_min_z(), mesh.get_max_z())).scale(sea_level);
    let river_cells = get_river_cells(&mesh, 256, before_sea_level, (0.01, 0.49), rng);

    mesh = mesh.rescale(&Scale::new(
        (mesh.get_min_z(), mesh.get_max_z()),
        (0.0, max_height),
    ));
    let terrain = mesh.get_z_vector().map(|z| z as f32);

    let mut out = World::new(terrain, sea_level as f32);

    for cell in river_cells {
        out.add_river(cell);
    }
    setup_temperatures(&mut out);
    out
}
